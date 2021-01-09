use core::sync::atomic::Ordering::Relaxed;

use radium::Radium;

use crate::scalar::{OpaqueScalar, ScalarAllocator};

use super::{Pool, PoolMut};

/// A [`Pool`] that iterates over it's elements and tries to put a given
/// [`OpaqueScalar`] into one of it's sub-pools. May fail if it can't put
/// an element into any of it's pools
///
/// i.e.
///
/// ```
/// # use std::cell::Cell; use pui_core::{pool::Sequence, scalar::OpaqueScalar, dynamic::Global as ThreadLocal};
/// let sequence = Sequence {
///     index: Cell::new(0),
///     pools: [Cell::new(None), Cell::new(None), Cell::new(None), Cell::new(None)],
/// };
/// let sequence: &Sequence<Cell<usize>, [Cell<Option<OpaqueScalar<_>>>]> = &sequence;
/// let dynamic = ThreadLocal::with_pool(sequence);
/// ```
pub struct Sequence<R, P: ?Sized> {
    /// The index must be either `Cell<usize>` or `AtomicUsize`
    pub index: R,
    /// The pools must have the type `[P]` where `P` is a pool
    ///
    /// This type is generic, so that it's possible to create it with safe code
    pub pools: P,
}

impl<A: ScalarAllocator, R: Radium<Item = usize>, P: PoolMut<A>> PoolMut<A> for Sequence<R, [P]> {
    fn insert_mut(&mut self, mut scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
        let index = self.index.get_mut();
        let initial_index = *index;
        let initial_index = initial_index % self.pools.len();

        loop {
            let current = *index % self.pools.len();

            if current == initial_index {
                return Some(scalar)
            }

            scalar = self.pools[current].insert_mut(scalar)?;
            *index = index.wrapping_add(1);
        }
    }

    fn remove_mut(&mut self) -> Option<OpaqueScalar<A>> {
        let index = self.index.get_mut();
        let initial_index = *index;
        let initial_index = initial_index % self.pools.len();

        loop {
            let current = *index % self.pools.len();

            if current == initial_index {
                return None
            }

            if let Some(scalar) = self.pools[current].remove_mut() {
                return Some(scalar)
            }

            *index = index.wrapping_add(1);
        }
    }
}

impl<A: ScalarAllocator, R: Radium<Item = usize>, P: Pool<A>> Pool<A> for Sequence<R, [P]> {
    fn insert(&self, mut scalar: OpaqueScalar<A>) -> Option<OpaqueScalar<A>> {
        let mut initial_index = None;
        loop {
            let current = self.index.fetch_add(1, Relaxed) % self.pools.len();
            match initial_index {
                None => initial_index = Some(current),
                Some(initial_index) => {
                    if current == initial_index {
                        return Some(scalar)
                    }
                }
            }

            scalar = self.pools[current].insert(scalar)?;
        }
    }

    fn remove(&self) -> Option<OpaqueScalar<A>> {
        let mut initial_index = None;
        loop {
            let current = self.index.fetch_add(1, Relaxed) % self.pools.len();
            match initial_index {
                None => initial_index = Some(current),
                Some(initial_index) => {
                    if current == initial_index {
                        return None
                    }
                }
            }

            if let Some(scalar) = self.pools[current].remove() {
                return Some(scalar)
            }
        }
    }
}
