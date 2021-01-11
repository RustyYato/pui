//! Hop Arenas - Fast Access, Fast Iteration, Slow Mutation, Small memory footprint
//!
//! A hop arena has a minimal footprint, it stores a doubly-linked-list of empty
//! slots embeded in the same location as the values, so as long as the size
//! of you values is greater than or equal to `[usize; 3]`, then there is no memory
//! overhead. This doubly-linked-list of empty slots means that insertion and deletion
//! are `O(1)` operations.
//!
//! This doubly-linked-list also encodes just enough inforation to about vacant slots
//! that you can skip over all contiguous vacant slots in one go while iterating. This
//! means if there are large blocks of vacant slots, a hop arena will efficiently jump
//! over the entire block in one go.
//!
//! While this method of iteration is faster than sparse arenas, it still can't compare
//! to dense arenas. However, hop arenas are more memory efficient than dense arenas.
//!
//! Each slot is versioned by using [`Version`] trait. See [`Version`] for docs
//! on version exhaustion. Once a slot's version exhausts, it will not be pushed
//! onto the doubly-linked list. This prevents it from ever being used again.

// FIXME - version exhaustion should be handled when iterating or inserting elements
// into the arena.

use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

use crate::{version::Version, ArenaAccess, BuildArenaKey};

mod imp;
use imp::Slot;
pub use imp::VacantEntry;

mod iter_unchecked;
use iter_unchecked::IteratorUnchecked;

/// A hop arena
#[derive(Debug, Clone)]
pub struct Arena<T, I = (), V: Version = crate::version::DefaultVersion> {
    slots: PuiVec<Slot<T, V>, I>,
    num_elements: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Arena<T> {
    /// Create a new arena
    pub fn new() -> Self { Self::with_ident(()) }
}

impl<T, V: Version> Arena<T, (), V> {
    /// Clear the arena without reducing it's capacity
    pub fn clear(&mut self) {
        self.slots.vec_mut().clear();
        let _: usize = self.slots.push(Slot::SENTINEL);
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    /// Create a new arena with the given identifier
    pub fn with_ident(ident: I) -> Self {
        Self {
            num_elements: 0,
            slots: PuiVec::from_raw_parts(std::vec![Slot::SENTINEL], ident),
        }
    }

    /// Get the associated identifier for this arena
    pub fn ident(&self) -> &I { self.slots.ident() }

    /// Returns true if the arena is empty
    pub fn is_empty(&self) -> bool { self.num_elements == 0 }

    /// Returns the number of elements in this arena
    pub fn len(&self) -> usize { self.num_elements }

    /// Returns the capacity of this arena
    pub fn capacity(&self) -> usize { self.slots.capacity() }

    /// Reserves capacity for at least additional more elements to be inserted
    /// in the given Arena<T>. The collection may reserve more space to avoid
    /// frequent reallocations. After calling reserve, capacity will be greater
    /// than or equal to self.len() + additional. Does nothing if capacity is
    /// already sufficient.
    pub fn reserve(&mut self, additional: usize) { self.slots.reserve(additional) }

    /// Check if an index is in bounds, and if it is return a `Key<_, _>` to it
    #[inline]
    pub fn parse_key<K: BuildArenaKey<I, V>>(&self, index: usize) -> Option<K> {
        let slot = self.slots.get(index)?;
        slot.parse_key(index, self.slots.ident())
    }

    /// Return a handle to a vacant entry allowing for further manipulation.
    ///
    /// This function is useful when creating values that must contain their
    /// key. The returned VacantEntry reserves a slot in the arena and is able
    /// to query the associated key.
    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I, V> { self.__vacant_entry() }

    /// Insert a value in the arena, returning key assigned to the value.
    ///
    /// The returned key can later be used to retrieve or remove the value
    /// using indexed lookup and remove. Additional capacity is allocated
    /// if needed.
    pub fn insert<K: BuildArenaKey<I, V>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    /// Return true if a value is associated with the given key.
    pub fn contains<K: ArenaAccess<I, V>>(&self, key: K) -> bool {
        let index = match key.validate_ident(self.ident(), crate::Validator::new()).into_inner() {
            Err(index) if self.slots.len() <= index => return false,
            Ok(index) | Err(index) => index,
        };

        let version = unsafe { self.slots.get_unchecked(index).version() };

        match key.version() {
            Some(saved) => version.equals_saved(saved),
            None => version.is_full(),
        }
    }

    /// Remove and return the value associated with the given key.
    ///
    /// The key is then released and may be associated with future stored values,
    /// if the versioning strategy allows it.
    ///
    /// Panics if key is not associated with a value.
    #[track_caller]
    pub fn remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove from an `Arena` using a stale `Key`")
    }

    /// Remove and return the value associated with the given key.
    ///
    /// The key is then released and may be associated with future stored values,
    /// if the versioning strategy allows it.
    ///
    /// Returns `None` if key is not associated with a value.
    pub fn try_remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<T> {
        if self.contains(&key) {
            Some(unsafe { self.remove_unchecked(key.index()) })
        } else {
            None
        }
    }

    /// Removes the value associated with the given key.
    ///
    /// The key is then released and may be associated with future stored values,
    /// if the versioning strategy allows it.
    ///
    /// Returns true if the value was removed, an false otherwise
    pub fn delete<K: ArenaAccess<I, V>>(&mut self, key: K) -> bool {
        if self.contains(&key) {
            unsafe { self.delete_unchecked(key.index()) }
            true
        } else {
            false
        }
    }

    /// Return a shared reference to the value associated with the given key.
    ///
    /// If the given key is not associated with a value, then None is returned.
    pub fn get<K: ArenaAccess<I, V>>(&self, key: K) -> Option<&T> {
        if self.contains(&key) {
            unsafe { Some(self.get_unchecked(key.index())) }
        } else {
            None
        }
    }

    /// Return a unique reference to the value associated with the given key.
    ///
    /// If the given key is not associated with a value, then None is returned.
    pub fn get_mut<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<&mut T> {
        if self.contains(&key) {
            unsafe { Some(self.get_unchecked_mut(key.index())) }
        } else {
            None
        }
    }

    /// Return a shared reference to the value associated with the
    /// given key without performing bounds checking, or checks
    /// if there is a value associated to the key
    ///
    /// # Safety
    ///
    /// `contains` should return true with the given index.
    pub unsafe fn get_unchecked(&self, index: usize) -> &T { self.slots.get_unchecked(index).get_unchecked() }

    /// Return a unique reference to the value associated with the
    /// given key without performing bounds checking, or checks
    /// if there is a value associated to the key
    ///
    /// # Safety
    ///
    /// `contains` should return true with the given index.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        self.slots.get_unchecked_mut(index).get_mut_unchecked()
    }

    /// Deletes all elements from the arena
    pub fn delete_all(&mut self) { self.retain(|_| false) }

    /// Retain only the elements specified by the predicate.
    ///
    /// If the predicate returns for a given element true,
    /// then the element is kept in the arena.
    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        let mut i = 0;

        for _ in 0..self.num_elements {
            unsafe {
                let slot = self.slots.get_unchecked_mut(i);
                if slot.is_vacant() {
                    i = 1 + slot.other_end();
                }

                let value = self.slots.get_unchecked_mut(i).get_mut_unchecked();

                if !f(value) {
                    self.delete_unchecked(i);
                }
            }

            i += 1;
        }
    }

    /// An iterator over the keys of the arena, in no particular order
    pub fn keys<K: BuildArenaKey<I, V>>(&self) -> Keys<'_, T, I, V, K> {
        Keys {
            entries: self.entries(),
        }
    }

    /// An iterator of shared references to values of the arena,
    /// in no particular order
    pub fn iter(&self) -> Iter<'_, T, V> {
        Iter {
            slots: Occupied {
                len: self.num_elements,
                slots: iter_unchecked::Iter::new(&self.slots).enumerate(),
            },
        }
    }

    /// An iterator of unique references to values of the arena,
    /// in no particular order
    pub fn iter_mut(&mut self) -> IterMut<'_, T, V> {
        IterMut {
            slots: OccupiedMut {
                len: self.num_elements,
                slots: iter_unchecked::IterMut::new(&mut self.slots).enumerate(),
            },
        }
    }

    fn cursor(&mut self) -> Cursor<'_, T, V> {
        Cursor {
            range: 0..self.slots.len(),
            slots: &mut self.slots,
            num_elements: &mut self.num_elements,
        }
    }

    /// Return a draining iterator that removes all elements from the
    /// arena and yields the removed items.
    ///
    /// Note: Elements are removed even if the iterator is only partially
    /// consumed or not consumed at all.
    pub fn drain(&mut self) -> Drain<'_, T, V> { Drain { cursor: self.cursor() } }

    /// Return a draining iterator that removes all elements specified by the predicate
    /// from the arena and yields the removed items.
    ///
    /// If the predicate returns true for a given element, then it is removed from
    /// the arena, and yielded from the iterator.
    ///
    /// Note: Elements are removed even if the iterator is only partially
    /// consumed or not consumed at all.
    pub fn drain_filter<F: FnMut(&mut T) -> bool>(&mut self, filter: F) -> DrainFilter<'_, T, V, F> {
        DrainFilter {
            cursor: self.cursor(),
            filter,
            panicked: false,
        }
    }

    /// An iterator of keys and shared references to values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn entries<K: BuildArenaKey<I, V>>(&self) -> Entries<'_, T, I, V, K> {
        Entries {
            slots: Occupied {
                len: self.num_elements,
                slots: iter_unchecked::Iter::new(&self.slots).enumerate(),
            },
            ident: self.slots.ident(),
            key: PhantomData,
        }
    }

    /// An iterator of keys and unique references to values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn entries_mut<K: BuildArenaKey<I, V>>(&mut self) -> EntriesMut<'_, T, I, V, K> {
        let (ident, slots) = self.slots.as_mut_parts();
        EntriesMut {
            slots: OccupiedMut {
                len: self.num_elements,
                slots: iter_unchecked::IterMut::new(slots).enumerate(),
            },
            ident,
            key: PhantomData,
        }
    }

    /// An iterator of keys and values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn into_entries<K: BuildArenaKey<I, V>>(self) -> IntoEntries<T, I, V, K> {
        let (ident, slots) = unsafe { self.slots.into_raw_parts() };
        IntoEntries {
            slots: IntoOccupied {
                len: self.num_elements,
                slots: iter_unchecked::IntoIter::new(slots).enumerate(),
            },
            ident,
            key: PhantomData,
        }
    }
}

impl<T, I, V: Version> IntoIterator for Arena<T, I, V> {
    type Item = T;
    type IntoIter = IntoIter<T, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            slots: IntoOccupied {
                len: self.num_elements,
                slots: iter_unchecked::IntoIter::new(unsafe { self.slots.into_raw_parts().1 }).enumerate(),
            },
        }
    }
}

impl<T, I, V: Version, K: ArenaAccess<I, V>> Index<K> for Arena<T, I, V> {
    type Output = T;

    #[track_caller]
    fn index(&self, key: K) -> &Self::Output { self.get(key).expect("Tried to access `Arena` with a stale `Key`") }
}

impl<T, I, V: Version, K: ArenaAccess<I, V>> IndexMut<K> for Arena<T, I, V> {
    #[track_caller]
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

struct OccupiedBase<I> {
    len: usize,
    slots: iter_unchecked::Enumerate<I>,
}

type Occupied<'a, T, V> = OccupiedBase<iter_unchecked::Iter<'a, Slot<T, V>>>;
type OccupiedMut<'a, T, V> = OccupiedBase<iter_unchecked::IterMut<'a, Slot<T, V>>>;
type IntoOccupied<T, V> = OccupiedBase<iter_unchecked::IntoIter<Slot<T, V>>>;

impl<I: IteratorUnchecked> Iterator for OccupiedBase<I> {
    type Item = (usize, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        unsafe {
            loop {
                let index = self.slots.index();
                let slot = self.slots.peek();
                if slot.is_vacant() {
                    let skip = slot.other_end().wrapping_sub(index).wrapping_add(1);
                    self.slots.advance(skip);
                } else {
                    return Some(self.slots.next())
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (self.len, Some(self.len)) }

    fn count(self) -> usize { self.len }
}

impl<I: IteratorUnchecked> DoubleEndedIterator for OccupiedBase<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        unsafe {
            loop {
                let index = self.slots.index_back();
                let slot = self.slots.peek_back();
                if slot.is_vacant() {
                    let skip = index.wrapping_sub(slot.other_end());
                    self.slots.advance_back(skip);
                } else {
                    return Some(self.slots.next_back())
                }
            }
        }
    }
}

/// Returned by [`Arena::keys`]
pub struct Keys<'a, T, I, V: Version, K> {
    entries: Entries<'a, T, I, V, K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Keys<'a, T, I, V, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> { self.entries.next().map(|(key, _)| key) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.entries.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.entries.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Keys<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.entries.next_back().map(|(key, _)| key) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for Keys<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Keys<'_, T, I, V, K> {}

#[inline(always)]
fn value<T, U, V>((_, (_, v)): (T, (U, V))) -> V { v }
#[inline(always)]
unsafe fn entry<I, V: Version, T, K: BuildArenaKey<I, V>>(ident: &I) -> impl '_ + FnOnce((usize, (V, T))) -> (K, T) {
    #[inline(always)]
    move |(index, (version, value))| (K::new_unchecked(index, version.save(), ident), value)
}

/// Returned by [`Arena::iter`]
pub struct Iter<'a, T, V: Version> {
    slots: Occupied<'a, T, V>,
}

impl<'a, T, V: Version> Iterator for Iter<'a, T, V> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(value) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, V: Version> DoubleEndedIterator for Iter<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(value) }
}
impl<T, V: Version> ExactSizeIterator for Iter<'_, T, V> {}
impl<T, V: Version> core::iter::FusedIterator for Iter<'_, T, V> {}

/// Returned by [`Arena::iter_mut`]
pub struct IterMut<'a, T, V: Version> {
    slots: OccupiedMut<'a, T, V>,
}

impl<'a, T, V: Version> Iterator for IterMut<'a, T, V> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(value) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, V: Version> DoubleEndedIterator for IterMut<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(value) }
}
impl<T, V: Version> ExactSizeIterator for IterMut<'_, T, V> {}
impl<T, V: Version> core::iter::FusedIterator for IterMut<'_, T, V> {}

/// Returned by [`Arena::into_iter`]
pub struct IntoIter<T, V: Version> {
    slots: IntoOccupied<T, V>,
}

impl<T, V: Version> Iterator for IntoIter<T, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(value) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, V: Version> DoubleEndedIterator for IntoIter<T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(value) }
}
impl<T, V: Version> ExactSizeIterator for IntoIter<T, V> {}
impl<T, V: Version> core::iter::FusedIterator for IntoIter<T, V> {}

struct Cursor<'a, T, V: Version> {
    slots: &'a mut [Slot<T, V>],
    num_elements: &'a mut usize,
    range: core::ops::Range<usize>,
}

impl<T, V: Version> Cursor<'_, T, V> {
    fn next(&mut self) -> Option<(usize, &mut T)> {
        let mut index = self.range.next()?;

        let slot = unsafe { self.slots.get_unchecked_mut(index) };
        if slot.is_vacant() {
            self.range.start = unsafe { slot.other_end() };
            index = self.range.next()?;
        }

        Some((index, unsafe { slot.get_mut_unchecked() }))
    }

    fn next_back(&mut self) -> Option<(usize, &mut T)> {
        let mut index = self.range.next_back()?;

        let slot = unsafe { self.slots.get_unchecked_mut(index) };
        if slot.is_vacant() {
            self.range.start = unsafe { slot.other_end() };
            index = self.range.next_back()?;
        }

        Some((index, unsafe { slot.get_mut_unchecked() }))
    }

    unsafe fn take(&mut self, index: usize) -> T {
        *self.num_elements -= 1;
        imp::remove_unchecked(self.slots, index)
    }
}

/// Returned by [`Arena::drain`]
pub struct Drain<'a, T, V: Version> {
    cursor: Cursor<'a, T, V>,
}

impl<T, V: Version> Drop for Drain<'_, T, V> {
    fn drop(&mut self) { self.for_each(drop); }
}

impl<T, V: Version> Iterator for Drain<'_, T, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let (index, _) = self.cursor.next()?;
        Some(unsafe { self.cursor.take(index) })
    }
}

impl<T, V: Version> DoubleEndedIterator for Drain<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (index, _) = self.cursor.next_back()?;
        Some(unsafe { self.cursor.take(index) })
    }
}

/// Returned by [`Arena::drain_filter`]
pub struct DrainFilter<'a, T, V: Version, F: FnMut(&mut T) -> bool> {
    cursor: Cursor<'a, T, V>,
    filter: F,
    panicked: bool,
}

impl<T, V: Version, F: FnMut(&mut T) -> bool> Drop for DrainFilter<'_, T, V, F> {
    fn drop(&mut self) {
        if !self.panicked {
            self.for_each(drop);
        }
    }
}

impl<'a, T, V: Version, F: FnMut(&mut T) -> bool> Iterator for DrainFilter<'a, T, V, F> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.cursor.next()?;
            let panicked = crate::SetOnDrop(&mut self.panicked);
            let return_value = (self.filter)(value);
            panicked.defuse();
            if return_value {
                return Some(unsafe { self.cursor.take(index) })
            }
        }
    }
}

impl<T, V: Version, F: FnMut(&mut T) -> bool> DoubleEndedIterator for DrainFilter<'_, T, V, F> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (index, value) = self.cursor.next_back()?;
            let panicked = crate::SetOnDrop(&mut self.panicked);
            let return_value = (self.filter)(value);
            panicked.defuse();
            if return_value {
                return Some(unsafe { self.cursor.take(index) })
            }
        }
    }
}

/// Returned by [`Arena::entries`]
pub struct Entries<'a, T, I, V: Version, K> {
    slots: Occupied<'a, T, V>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Entries<'a, T, I, V, K> {
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(unsafe { entry(self.ident) }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Entries<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(unsafe { entry(self.ident) }) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for Entries<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Entries<'_, T, I, V, K> {}

/// Returned by [`Arena::entries_mut`]
pub struct EntriesMut<'a, T, I, V: Version, K> {
    slots: OccupiedMut<'a, T, V>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for EntriesMut<'a, T, I, V, K> {
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(unsafe { entry(self.ident) }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for EntriesMut<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(unsafe { entry(self.ident) }) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for EntriesMut<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for EntriesMut<'_, T, I, V, K> {}

/// Returned by [`Arena::into_entries`]
pub struct IntoEntries<T, I, V: Version, K> {
    slots: IntoOccupied<T, V>,
    ident: I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for IntoEntries<T, I, V, K> {
    type Item = (K, T);

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(unsafe { entry(&self.ident) }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for IntoEntries<T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(unsafe { entry(&self.ident) }) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for IntoEntries<T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for IntoEntries<T, I, V, K> {}

#[cfg(test)]
mod test {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn basic() {
        let mut arena = Arena::new();

        let a: usize = arena.insert(0);
        assert_eq!(a, 1);
        assert_eq!(arena[a], 0);
        assert_eq!(arena.remove(a), 0);
        assert_eq!(arena.get(a), None);

        let b: usize = arena.insert(10);
        assert_eq!(b, 1);
        assert_eq!(arena[b], 10);

        let b: usize = arena.insert(20);
        assert_eq!(b, 2);
        assert_eq!(arena[b], 20);
        assert_eq!(arena.remove(a), 10);
        assert_eq!(arena.get(a), None);

        let c: usize = arena.insert(30);
        assert_eq!(c, 1);
        assert_eq!(arena[c], 30);
        assert_eq!(arena.remove(b), 20);
        assert_eq!(arena.get(b), None);
        assert_eq!(arena.remove(c), 30);
        assert_eq!(arena.get(c), None);
    }

    #[test]
    fn basic_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        for i in ins_values.len()..10 {
            ins_values.push(arena.insert(i * 100));
        }
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn zero_sized() {
        let mut arena = Arena::new();

        let a: usize = arena.insert(());
        let b: usize = arena.insert(());
        let c: usize = arena.insert(());
        let d: usize = arena.insert(());
        let e: usize = arena.insert(());

        arena.remove(b);
        arena.remove(d);
        arena.remove(a);
        arena.remove(c);
        arena.remove(e);

        let a: usize = arena.insert(());
        let b: usize = arena.insert(());
        let c: usize = arena.insert(());
        let d: usize = arena.insert(());
        let e: usize = arena.insert(());

        arena.remove(b);
        arena.remove(d);
        arena.remove(a);
        arena.remove(c);
        arena.remove(e);
    }

    #[test]
    fn basic_retain() {
        let mut arena = Arena::new();
        for i in 0..10 {
            let _: usize = arena.insert(i);
        }
        arena.retain(|&mut i| i % 3 == 0);
        let mut items = arena.iter().copied().collect::<Vec<_>>();
        items.sort_unstable();
        assert_eq!(items, [0, 3, 6, 9]);
    }

    #[test]
    fn iter_keys_insert_only() {
        let mut arena = Arena::new();
        let ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_keys = arena.keys().collect::<Vec<usize>>();
        iter_keys.sort_unstable();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_rev_insert_only() {
        let mut arena = Arena::new();
        let ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_keys = arena.keys().rev().collect::<Vec<usize>>();
        iter_keys.sort_unstable();

        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_with_removal() {
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        let mut iter_keys = arena.keys().collect::<Vec<usize>>();
        iter_keys.sort_unstable();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        ins_keys.sort_unstable();
        let mut iter_keys = arena.keys().rev().collect::<Vec<usize>>();
        iter_keys.sort_unstable();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_with_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        for i in ins_keys.len()..10 {
            ins_keys.push(arena.insert(i * 100));
        }
        let mut iter_keys = arena.keys().collect::<Vec<usize>>();
        let mut rev_iter_keys = arena.keys().rev().collect::<Vec<usize>>();

        // the order that the keys come out doesn't matter,
        // so put them in a canonical order
        ins_keys.sort_unstable();
        iter_keys.sort_unstable();
        rev_iter_keys.sort_unstable();

        assert_eq!(ins_keys, iter_keys);
        assert_eq!(ins_keys, rev_iter_keys);
    }

    #[test]
    fn iter_values_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values = arena.iter().copied().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_rev_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values = arena.iter().copied().rev().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values = arena.iter().copied().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values = arena.iter().copied().rev().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_with_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        for i in ins_values.len()..10 {
            ins_values.push(arena.insert(i * 100));
        }
        let mut iter_values = arena.iter().copied().collect::<Vec<usize>>();
        let mut rev_iter_values = arena.iter().copied().rev().collect::<Vec<usize>>();

        // the order that the iter come out doesn't matter,
        // so put them in a canonical order
        iter_values.sort_unstable();
        rev_iter_values.sort_unstable();

        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
        assert_eq!(rev_iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
    }

    #[test]
    fn iter_values_mut_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_mut_rev_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_mut_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_mut_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_mut_with_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        for i in ins_values.len()..10 {
            ins_values.push(arena.insert(i * 100));
        }
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<usize>>();
        let mut rev_iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<usize>>();

        // the order that the iter come out doesn't matter,
        // so put them in a canonical order
        iter_values_mut.sort_unstable();
        rev_iter_values_mut.sort_unstable();

        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
        assert_eq!(rev_iter_values_mut, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
    }

    #[test]
    fn into_iter_values_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut into_iter_values = arena.into_iter().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn into_iter_values_rev_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut into_iter_values = arena.into_iter().rev().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn into_iter_values_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut into_iter_values = arena.into_iter().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn into_iter_values_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut into_iter_values = arena.into_iter().rev().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn into_iter_values_with_reinsertion() {
        let mk_arena = || {
            let mut arena = Arena::new();
            let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
            for i in (0..ins_values.len()).rev().step_by(3) {
                let key = ins_values.remove(i);
                arena.remove(key);
            }
            for i in ins_values.len()..10 {
                ins_values.push(arena.insert(i * 100));
            }
            arena
        };
        let mut into_iter_values = mk_arena().into_iter().collect::<Vec<usize>>();
        let mut rev_into_iter_values = mk_arena().into_iter().rev().collect::<Vec<usize>>();

        // the order that the iter come out doesn't matter,
        // so put them in a canonical order
        into_iter_values.sort_unstable();
        rev_into_iter_values.sort_unstable();

        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
        assert_eq!(rev_into_iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
    }
}
