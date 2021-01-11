//! A pool of ids that can be used to reuse ids in [`Dynamic`](crate::dynamic::Dynamic).

use crate::scalar::{OpaqueScalar, ScalarAllocator};

mod ext;
mod flag;
mod sequence;

pub use flag::Flag;
pub use sequence::Sequence;

#[doc(hidden)]
pub mod export {
    #[cfg(feature = "std")]
    pub use super::ext::LocalKey;
}

#[doc(hidden)]
#[macro_export]
macro_rules! __global_pool {
    ($name:ident($imp:ty)) => {
        impl $crate::Init for $name {
            const INIT: Self = Self;
        }

        const _: () = {
            static __IMP_POOL: $imp = $crate::Init::INIT;

            impl<A: $crate::scalar::ScalarAllocator> $crate::pool::PoolMut<A> for $name
            where
                $imp: $crate::pool::Pool<A>,
            {
                fn insert_mut(
                    &mut self,
                    scalar: $crate::scalar::OpaqueScalar<A>,
                ) -> Option<$crate::scalar::OpaqueScalar<A>> {
                    $crate::pool::Pool::insert(&__IMP_POOL, scalar)
                }

                fn remove_mut(&mut self) -> Option<$crate::scalar::OpaqueScalar<A>> {
                    $crate::pool::Pool::remove(&__IMP_POOL)
                }
            }

            impl<A: $crate::scalar::ScalarAllocator> $crate::pool::Pool<A> for $name
            where
                $imp: $crate::pool::Pool<A>,
            {
                fn insert(&self, scalar: $crate::scalar::OpaqueScalar<A>) -> Option<$crate::scalar::OpaqueScalar<A>> {
                    $crate::pool::Pool::insert(&__IMP_POOL, scalar)
                }

                fn remove(&self) -> Option<$crate::scalar::OpaqueScalar<A>> { $crate::pool::Pool::remove(&__IMP_POOL) }
            }
        };
    };
    (thread_local $name:ident($imp:ty)) => {
        impl $crate::Init for $name {
            const INIT: Self = Self;
        }

        const _: () = {
            $crate::export::thread_local! {
                static __IMP_POOL: $imp = $crate::export::Default::default();
            }

            impl<A: $crate::scalar::ScalarAllocator> $crate::pool::PoolMut<A> for $name
            where
                $imp: $crate::pool::Pool<A>,
            {
                fn insert_mut(
                    &mut self,
                    scalar: $crate::scalar::OpaqueScalar<A>,
                ) -> Option<$crate::scalar::OpaqueScalar<A>> {
                    $crate::pool::Pool::insert(&$crate::pool::export::LocalKey(&__IMP_POOL), scalar)
                }

                fn remove_mut(&mut self) -> Option<$crate::scalar::OpaqueScalar<A>> {
                    $crate::pool::Pool::remove(&$crate::pool::export::LocalKey(&__IMP_POOL))
                }
            }

            impl<A: $crate::scalar::ScalarAllocator> $crate::pool::Pool<A> for $name
            where
                $imp: $crate::pool::Pool<A>,
            {
                fn insert(&self, scalar: $crate::scalar::OpaqueScalar<A>) -> Option<$crate::scalar::OpaqueScalar<A>> {
                    $crate::pool::Pool::insert(&$crate::pool::export::LocalKey(&__IMP_POOL), scalar)
                }

                fn remove(&self) -> Option<$crate::scalar::OpaqueScalar<A>> {
                    $crate::pool::Pool::remove(&$crate::pool::export::LocalKey(&__IMP_POOL))
                }
            }
        };
    };
}

/// Create a new type that implements [`Pool`](crate::pool::Pool) and [`PoolMut`](crate::pool::PoolMut)
/// that can be used with [`Dynamic`](crate::dynamic::Dynamic)
///
/// For example,
///
/// ```
/// # #[cfg(feature = "std")]
/// pui_core::global_pool! {
///     pub struct MyPool(pui_core::pool::SyncStackPool<pui_core::dynamic::Global>);
/// }
///
/// let _my_pool = MyPool;
/// ```
///
/// will generate a global pool that is backed by a `SyncStackPool`, and holds `Global`s
#[macro_export]
macro_rules! global_pool {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident($imp:ty);
    ) => {
        $(#[$meta])*
        $v struct $name;

        $crate::__global_pool!{$name($imp)}
    };
    (
        $(#[$meta:meta])*
        $v:vis thread_local struct $name:ident($imp:ty);
    ) => {
        $(#[$meta])*
        $v struct $name;

        $crate::__global_pool!{thread_local $name($imp)}
    };
}

#[cfg(feature = "alloc")]
cfg_if::cfg_if! {
    if #[cfg(feature = "parking_lot")] {
        use parking_lot::Mutex;
        use std::vec::Vec;
        use std::collections::VecDeque;

        /// A thread safe stack-pool, it returns scalars in LIFO order
        #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
        pub struct SyncStackPool<T: ScalarAllocator>(Mutex<Vec<crate::scalar::OpaqueScalar<T>>>);

        /// A thread safe queue-pool, it returns scalars in FIFO order
        #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
        pub struct SyncQueuePool<T: ScalarAllocator>(once_cell::sync::Lazy<Mutex<VecDeque<crate::scalar::OpaqueScalar<T>>>>);
    } else if #[cfg(feature = "std")] {
        use std::sync::Mutex;
        use std::collections::VecDeque;

        /// A thread safe stack-pool, it returns scalars in LIFO order
        #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
        pub struct SyncStackPool<T: ScalarAllocator>(once_cell::sync::Lazy<Mutex<Vec<crate::scalar::OpaqueScalar<T>>>>);

        /// A thread safe queue-pool, it returns scalars in FIFO order
        #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
        pub struct SyncQueuePool<T: ScalarAllocator>(once_cell::sync::Lazy<Mutex<VecDeque<crate::scalar::OpaqueScalar<T>>>>);
    }
}

/// A pool of ids that can be used to reuse ids in [`Dynamic`](crate::dynamic::Dynamic).
pub trait PoolMut<A: ScalarAllocator> {
    /// Put a new id into the pool
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>>;

    /// Take an id out of the pool
    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>>;
}

/// A pool of ids that can be used to reuse ids in [`Dynamic`](crate::dynamic::Dynamic).
pub trait Pool<A: ScalarAllocator>: PoolMut<A> {
    /// Put a new id into the pool
    fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>>;

    /// Take an id out of the pool
    fn remove(&self) -> Option<OpaqueScalar<A>>;
}

impl crate::Init for () {
    const INIT: Self = ();
}

impl<A: ScalarAllocator> PoolMut<A> for () {
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { Some(scalar) }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { None }
}

impl<A: ScalarAllocator> Pool<A> for () {
    fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { Some(scalar) }

    fn remove(&self) -> Option<OpaqueScalar<A>> { None }
}

impl<P: ?Sized + PoolMut<A>, A: ScalarAllocator> PoolMut<A> for &mut P {
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert_mut(self, scalar) }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { P::remove_mut(self) }
}

impl<P: ?Sized + Pool<A>, A: ScalarAllocator> Pool<A> for &mut P {
    fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert(self, scalar) }

    fn remove(&self) -> Option<OpaqueScalar<A>> { P::remove(self) }
}

impl<P: ?Sized + Pool<A>, A: ScalarAllocator> PoolMut<A> for &P {
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert(self, scalar) }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> { P::remove(self) }
}

impl<P: ?Sized + Pool<A>, A: ScalarAllocator> Pool<A> for &P {
    fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> { P::insert(self, scalar) }

    fn remove(&self) -> Option<OpaqueScalar<A>> { P::remove(self) }
}
