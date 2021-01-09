use core::{
    cell::{Cell, RefCell},
    mem::ManuallyDrop,
};

use super::{Pool, PoolMut};
use crate::{
    scalar::{OpaqueScalar, ScalarAllocator},
    Init,
};

struct Take<'a, T>(&'a Cell<T>, ManuallyDrop<T>);

impl<T> Drop for Take<'_, T> {
    fn drop(&mut self) { self.0.set(unsafe { ManuallyDrop::take(&mut self.1) }) }
}

impl<T: Init> Init for Cell<T> {
    const INIT: Self = Cell::new(T::INIT);
}

impl<A: ScalarAllocator, T: ?Sized + PoolMut<A>> PoolMut<A> for Cell<T> {
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.get_mut().insert_mut(scalar) }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.get_mut().remove_mut() }
}

impl<A: ScalarAllocator, T: Default + PoolMut<A>> Pool<A> for Cell<T> {
    fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
        let inner = self.take();
        let mut inner = Take(self, ManuallyDrop::new(inner));
        let inner = &mut *inner.1;
        inner.insert_mut(scalar)
    }

    fn remove(&self) -> Option<OpaqueScalar<A>> {
        let inner = self.take();
        let mut inner = Take(self, ManuallyDrop::new(inner));
        let inner = &mut *inner.1;
        inner.remove_mut()
    }
}

impl<T: Init> Init for RefCell<T> {
    const INIT: Self = RefCell::new(T::INIT);
}

impl<A: ScalarAllocator, T: ?Sized + PoolMut<A>> PoolMut<A> for RefCell<T> {
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.get_mut().insert_mut(scalar) }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.get_mut().remove_mut() }
}

impl<A: ScalarAllocator, T: PoolMut<A>> Pool<A> for RefCell<T> {
    fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
        match self.try_borrow_mut() {
            Ok(mut inner) => inner.insert_mut(scalar),
            Err(_) => Some(scalar),
        }
    }

    fn remove(&self) -> Option<OpaqueScalar<A>> { self.try_borrow_mut().ok()?.remove_mut() }
}

impl<T> Init for Option<T> {
    const INIT: Self = None;
}

impl<A: ScalarAllocator> PoolMut<A> for Option<OpaqueScalar<A>> {
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.replace(scalar) }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.take() }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "alloc")] {
        impl<T> Init for std::vec::Vec<T> {
            const INIT: Self = std::vec::Vec::new();
        }

        impl<A: ScalarAllocator> PoolMut<A> for std::vec::Vec<OpaqueScalar<A>> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
                self.push(scalar);
                None
            }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.pop() }
        }

        impl<A: ScalarAllocator> PoolMut<A> for std::collections::VecDeque<OpaqueScalar<A>> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
                self.push_back(scalar);
                None
            }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.pop_front() }
        }

        impl<A: ScalarAllocator> PoolMut<A> for std::collections::BinaryHeap<OpaqueScalar<A>> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
                self.push(scalar);
                None
            }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.pop() }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {

        #[doc(hidden)]
        pub struct LocalKey<P: 'static>(pub &'static std::thread::LocalKey<P>);

        impl<A: ScalarAllocator, P: Pool<A>> PoolMut<A> for LocalKey<P> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.insert(scalar) }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.remove() }
        }

        impl<A: ScalarAllocator, P: Pool<A>> Pool<A> for LocalKey<P> {
            fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.0.with(|pool| pool.insert(scalar)) }

            fn remove(&self) -> Option<OpaqueScalar<A>> { self.0.with(P::remove) }
        }

        impl<A: ScalarAllocator, P: PoolMut<A>> PoolMut<A> for std::sync::Mutex<P> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
                self.get_mut().ok()?.insert_mut(scalar)
            }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.get_mut().ok()?.remove_mut() }
        }

        impl<A: ScalarAllocator, P: PoolMut<A>> Pool<A> for std::sync::Mutex<P> {
            fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.lock().ok()?.insert_mut(scalar) }

            fn remove(&self) -> Option<OpaqueScalar<A>> { self.lock().ok()?.remove_mut() }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "parking_lot")] {
        impl<P: Init> Init for parking_lot::Mutex<P> {
            const INIT: Self = parking_lot::Mutex::const_new(parking_lot::lock_api::RawMutex::INIT, P::INIT);
        }

        impl<A: ScalarAllocator, P: PoolMut<A>> PoolMut<A> for parking_lot::Mutex<P> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.get_mut().insert_mut(scalar) }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.get_mut().remove_mut() }
        }

        impl<A: ScalarAllocator, P: PoolMut<A>> Pool<A> for parking_lot::Mutex<P> {
            fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.lock().insert_mut(scalar) }

            fn remove(&self) -> Option<OpaqueScalar<A>> { self.lock().remove_mut() }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(any(feature = "parking_lot", feature = "std"))] {
        impl<A: ScalarAllocator> Init for super::SyncStackPool<A> {
            const INIT: Self = super::SyncStackPool(Init::INIT);
        }

        impl<A: ScalarAllocator> PoolMut<A> for super::SyncStackPool<A> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.0.insert_mut(scalar) }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.0.remove_mut() }
        }

        impl<A: ScalarAllocator> Pool<A> for super::SyncStackPool<A> {
            fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.0.insert(scalar) }

            fn remove(&self) -> Option<OpaqueScalar<A>> { self.0.remove() }
        }

        impl<A: ScalarAllocator> Init for super::SyncQueuePool<A> {
            const INIT: Self = super::SyncQueuePool(Init::INIT);
        }

        impl<A: ScalarAllocator> PoolMut<A> for super::SyncQueuePool<A> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.0.insert_mut(scalar) }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { self.0.remove_mut() }
        }

        impl<A: ScalarAllocator> Pool<A> for super::SyncQueuePool<A> {
            fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { self.0.insert(scalar) }

            fn remove(&self) -> Option<OpaqueScalar<A>> { self.0.remove() }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "once_cell")] {
        impl<P: Default> Init for once_cell::sync::Lazy<P> {
            const INIT: Self = Self::new(P::default);
        }

        impl<A: ScalarAllocator, P: Pool<A>, F: FnOnce() -> P> PoolMut<A> for once_cell::sync::Lazy<P, F> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert_mut(self, scalar) }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { P::remove_mut(self) }
        }

        impl<A: ScalarAllocator, P: Pool<A>, F: FnOnce() -> P> Pool<A> for once_cell::sync::Lazy<P, F> {
            fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert(self, scalar) }

            fn remove(&self) -> Option<OpaqueScalar<A>> { P::remove(self) }
        }

        impl<P: Default> Init for once_cell::unsync::Lazy<P> {
            const INIT: Self = Self::new(P::default);
        }

        impl<A: ScalarAllocator, P: Pool<A>, F: FnOnce() -> P> PoolMut<A> for once_cell::unsync::Lazy<P, F> {
            fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert_mut(self, scalar) }

            fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { P::remove_mut(self) }
        }

        impl<A: ScalarAllocator, P: Pool<A>, F: FnOnce() -> P> Pool<A> for once_cell::unsync::Lazy<P, F> {
            fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert(self, scalar) }

            fn remove(&self) -> Option<OpaqueScalar<A>> { P::remove(self) }
        }
    }
}
