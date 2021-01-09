use core::{
    marker::PhantomData,
    mem,
    sync::atomic::{Ordering::*, *},
};

use crate::{
    pool::{Pool, PoolMut},
    scalar::{OpaqueScalar, ScalarAllocator},
    Init,
};

/// A [`Pool`] that can hold one scalar element of type `()`
pub struct Flag<A> {
    flag: AtomicBool,
    alloc: PhantomData<A>,
}

impl<A> Init for Flag<A> {
    const INIT: Self = Self::new();
}

impl<A> Flag<A> {
    /// Create a new `Flag`
    pub const fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
            alloc: PhantomData,
        }
    }
}

impl<A: ScalarAllocator<Scalar = ()>> PoolMut<A> for Flag<A> {
    fn insert_mut(&mut self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
        let filled = self.flag.get_mut();
        if mem::replace(filled, true) {
            Some(scalar)
        } else {
            None
        }
    }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> {
        let filled = self.flag.get_mut();
        if mem::replace(filled, false) {
            Some(unsafe { OpaqueScalar::new(()) })
        } else {
            None
        }
    }
}

impl<A: ScalarAllocator<Scalar = ()>> Pool<A> for Flag<A> {
    fn insert(&self, scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
        if self.flag.swap(true, Acquire) {
            Some(scalar)
        } else {
            None
        }
    }

    fn remove(&self) -> Option<OpaqueScalar<A>> {
        if self.flag.swap(false, Release) {
            Some(unsafe { OpaqueScalar::new(()) })
        } else {
            None
        }
    }
}
