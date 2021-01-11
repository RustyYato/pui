//! Sparse Arenas - Fast Access, Slow Iteration, Fast Mutation, Small memory footprint
//!
//! A sparse arena has a minimal footprint, it stores a linked-list of empty
//! slots embeded in the same location as the values, so as long as the size
//! of you values is greater than or equal to `usize`, then there is no memory
//! overhead. This linked-list of empty slots means that insertion and deletion
//! are `O(1)` operations.
//!
//! Each slot is versioned by using [`Version`] trait. See [`Version`] for docs
//! on version exhaustion. Once a slot's version exhausts, it will not be pushed
//! onto the linked list. This prevents it from ever being used again.

use core::{
    marker::PhantomData,
    mem::{replace, ManuallyDrop},
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

use crate::{
    version::{DefaultVersion, Version},
    ArenaAccess, BuildArenaKey,
};

union Data<T> {
    value: ManuallyDrop<T>,
    next: usize,
}

struct Slot<T, V: Version> {
    version: V,
    data: Data<T>,
}

/// A sparse arena
#[derive(Debug, Clone)]
pub struct Arena<T, I = (), V: Version = DefaultVersion> {
    slots: PuiVec<Slot<T, V>, I>,
    next: usize,
    num_elements: usize,
}

/// An empty slot in a sparse arena
pub struct VacantEntry<'a, T, I, V: Version = DefaultVersion> {
    arena: &'a mut Arena<T, I, V>,
    new_next: usize,
}

impl<T, V: Version> Drop for Slot<T, V> {
    fn drop(&mut self) {
        if self.version.is_full() {
            unsafe { ManuallyDrop::drop(&mut self.data.value) }
        }
    }
}

impl<T, V: Version> Slot<T, V> {
    unsafe fn remove_unchecked(&mut self, index: usize, next: &mut usize) -> T {
        let value = ManuallyDrop::take(&mut self.data.value);
        match self.version.mark_empty() {
            Ok(next_version) => {
                self.version = next_version;

                self.data = Data {
                    next: replace(next, index),
                };
            }
            Err(next_version) => self.version = next_version,
        }

        value
    }

    unsafe fn delete_unchecked(&mut self, index: usize, next: &mut usize) {
        struct Fixup<'a, T, V: Version>(&'a mut Slot<T, V>, usize, &'a mut usize);

        impl<T, V: Version> Drop for Fixup<'_, T, V> {
            fn drop(&mut self) {
                let Self(ref mut slot, index, ref mut next) = *self;
                match unsafe { slot.version.mark_empty() } {
                    Err(next_version) => slot.version = next_version,
                    Ok(next_version) => {
                        slot.version = next_version;

                        slot.data = Data {
                            next: replace(next, index),
                        };
                    }
                }
            }
        }

        let fixup = Fixup(self, index, next);

        ManuallyDrop::drop(&mut fixup.0.data.value);
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Arena<T> {
    /// Create a new arena
    pub const fn new() -> Self { Self::INIT }
}

impl<T, V: Version> Arena<T, (), V> {
    /// An empty arena
    pub const INIT: Self = Self {
        slots: PuiVec::new(()),
        next: 0,
        num_elements: 0,
    };

    /// Clear the arena without reducing it's capacity
    pub fn clear(&mut self) {
        self.next = 0;
        self.slots.vec_mut().clear();
    }
}

impl<T, I, V: Version> VacantEntry<'_, T, I, V> {
    /// Get the key associated with the `VacantEntry`, this key can be used
    /// once this `VacantEntry` gets filled
    pub fn key<K: BuildArenaKey<I, V>>(&self) -> K {
        unsafe {
            K::new_unchecked(
                self.arena.next,
                self.arena
                    .slots
                    .get_unchecked(self.arena.next)
                    .version
                    .mark_full()
                    .save(),
                self.arena.ident(),
            )
        }
    }

    /// Insert an element into the vacant entry
    pub fn insert<K: BuildArenaKey<I, V>>(self, value: T) -> K {
        let slot = unsafe { self.arena.slots.get_unchecked_mut(self.arena.next) };
        slot.data = Data {
            value: ManuallyDrop::new(value),
        };
        slot.version = unsafe { slot.version.mark_full() };
        let version = unsafe { slot.version.save() };
        let index = self.arena.next;
        self.arena.next = self.new_next;
        self.arena.num_elements += 1;

        unsafe { K::new_unchecked(index, version, self.arena.ident()) }
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    /// Create a new arena with the given identifier
    pub fn with_ident(ident: I) -> Self {
        Self {
            slots: PuiVec::new(ident),
            next: 0,
            num_elements: 0,
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
    pub fn reserve(&mut self, additional: usize) {
        if let Some(additional) = self.capacity().wrapping_sub(self.num_elements).checked_sub(additional) {
            self.slots.reserve(additional)
        }
    }

    /// Check if an index is in bounds, and if it is return a `Key<_, _>` to it
    #[inline]
    pub fn parse_key<K: BuildArenaKey<I, V>>(&self, index: usize) -> Option<K> {
        let slot = self.slots.get(index)?;
        if slot.version.is_full() {
            Some(unsafe { K::new_unchecked(index, slot.version.save(), self.slots.ident()) })
        } else {
            None
        }
    }

    /// Return a handle to a vacant entry allowing for further manipulation.
    ///
    /// This function is useful when creating values that must contain their
    /// key. The returned VacantEntry reserves a slot in the arena and is able
    /// to query the associated key.
    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I, V> {
        #[cold]
        #[inline(never)]
        pub fn allocate_vacant_slot<T, I, V: Version>(this: &mut Arena<T, I, V>) {
            this.next = this.slots.len();
            let _: usize = this.slots.push(Slot {
                version: V::EMPTY,
                data: Data {
                    next: this.next.wrapping_add(1),
                },
            });
        }

        if self.slots.len() == self.next {
            allocate_vacant_slot(self);
        }

        let slot = unsafe { self.slots.get_unchecked_mut(self.next) };

        VacantEntry {
            new_next: unsafe { slot.data.next },
            arena: self,
        }
    }

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

        let version = unsafe { self.slots.get_unchecked(index).version };

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
            let index = key.index();
            Some(unsafe { self.remove_unchecked(index) })
        } else {
            None
        }
    }

    unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        self.num_elements -= 1;
        self.slots
            .get_unchecked_mut(index)
            .remove_unchecked(index, &mut self.next)
    }

    /// Removes the value associated with the given key.
    ///
    /// The key is then released and may be associated with future stored values,
    /// if the versioning strategy allows it.
    ///
    /// Returns true if the value was removed, an false otherwise
    pub fn delete<K: ArenaAccess<I, V>>(&mut self, key: K) -> bool {
        if self.contains(&key) {
            unsafe {
                self.delete_unchecked(key.index());
                true
            }
        } else {
            false
        }
    }

    pub(crate) unsafe fn delete_unchecked(&mut self, index: usize) {
        self.num_elements -= 1;
        self.slots
            .get_unchecked_mut(index)
            .delete_unchecked(index, &mut self.next)
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
    pub unsafe fn get_unchecked(&self, index: usize) -> &T { &*self.slots.get_unchecked(index).data.value }

    /// Return a unique reference to the value associated with the
    /// given key without performing bounds checking, or checks
    /// if there is a value associated to the key
    ///
    /// # Safety
    ///
    /// `contains` should return true with the given index.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        &mut *self.slots.get_unchecked_mut(index).data.value
    }

    /// Deletes all elements from the arena
    pub fn delete_all(&mut self) { self.retain(|_| false) }

    /// Retain only the elements specified by the predicate.
    ///
    /// If the predicate returns for a given element true,
    /// then the element is kept in the arena.
    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        for i in 0..self.slots.len() {
            if let Some(value) = self.get_mut(unsafe { crate::TrustedIndex::new(i) }) {
                if !f(value) {
                    unsafe {
                        self.slots.get_unchecked_mut(i).delete_unchecked(i, &mut self.next);
                    }
                }
            }
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
                slots: self.slots.iter(),
            },
        }
    }

    /// An iterator of unique references to values of the arena,
    /// in no particular order
    pub fn iter_mut(&mut self) -> IterMut<'_, T, V> {
        IterMut {
            slots: Occupied {
                slots: self.slots.iter_mut(),
            },
        }
    }

    /// Return a draining iterator that removes all elements from the
    /// arena and yields the removed items.
    ///
    /// Note: Elements are removed even if the iterator is only partially
    /// consumed or not consumed at all.
    pub fn drain(&mut self) -> Drain<'_, T, V> {
        Drain {
            slots: Occupied {
                slots: self.slots.iter_mut().enumerate(),
            },
            next: &mut self.next,
            num_elements: &mut self.num_elements,
        }
    }

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
            slots: Occupied {
                slots: self.slots.iter_mut().enumerate(),
            },
            next: &mut self.next,
            num_elements: &mut self.num_elements,
            filter,
            panicked: false,
        }
    }

    /// An iterator of keys and shared references to values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn entries<K: BuildArenaKey<I, V>>(&self) -> Entries<'_, T, I, V, K> {
        let ident = self.ident();

        Entries {
            slots: Occupied {
                slots: self.slots.iter().enumerate(),
            },
            ident,
            key: PhantomData,
        }
    }

    /// An iterator of keys and unique references to values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn entries_mut<K: BuildArenaKey<I, V>>(&mut self) -> EntriesMut<'_, T, I, V, K> {
        let (ident, slots) = self.slots.as_mut_parts();

        EntriesMut {
            slots: Occupied {
                slots: slots.iter_mut().enumerate(),
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
            slots: Occupied {
                slots: slots.into_iter().enumerate(),
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
            slots: Occupied {
                slots: unsafe { self.slots.into_raw_parts().1.into_iter() },
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

impl<T, I, V: Version> Extend<T> for Arena<T, I, V> {
    #[allow(clippy::drop_copy)]
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        iter.for_each(move |value| drop::<usize>(self.vacant_entry().insert(value)));
    }
}

use core::fmt;

impl<T: Clone, V: Version> Clone for Slot<T, V> {
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            data: if self.version.is_full() {
                Data {
                    value: unsafe { self.data.value.clone() },
                }
            } else {
                Data {
                    next: unsafe { self.data.next },
                }
            },
        }
    }

    fn clone_from(&mut self, source: &Self) {
        if self.version.is_full() && source.version.is_full() {
            self.version = source.version;
            unsafe {
                self.data.value.clone_from(&source.data.value);
            }
        } else {
            *self = source.clone()
        }
    }
}

impl<T: fmt::Debug, V: Version + fmt::Debug> fmt::Debug for Slot<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.version.is_full() {
            f.debug_struct("Occupied")
                .field("version", &self.version)
                .field("value", unsafe { &*self.data.value })
                .finish()
        } else {
            f.debug_struct("Vacant")
                .field("version", &self.version)
                .field("next", unsafe { &self.data.next })
                .finish()
        }
    }
}

struct Occupied<I> {
    slots: I,
}

trait AsSlot {
    type Item;
    type Version: Version;

    fn as_slot(&self) -> &Slot<Self::Item, Self::Version>;
}

impl<T, V: Version> AsSlot for Slot<T, V> {
    type Item = T;
    type Version = V;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self }
}

impl<T, V: Version> AsSlot for &mut Slot<T, V> {
    type Item = T;
    type Version = V;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self }
}

impl<T, V: Version> AsSlot for &Slot<T, V> {
    type Item = T;
    type Version = V;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self }
}

impl<T: AsSlot> AsSlot for (usize, T) {
    type Item = T::Item;
    type Version = T::Version;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self.1.as_slot() }
}

impl<I: Iterator> Iterator for Occupied<I>
where
    I::Item: AsSlot,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> { self.slots.by_ref().find(|slot| slot.as_slot().version.is_full()) }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Occupied<I>
where
    I::Item: AsSlot,
{
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.by_ref().rfind(|slot| slot.as_slot().version.is_full()) }
}

/// Returned by [`Arena::keys`]
pub struct Keys<'a, T, I, V: Version, K> {
    entries: Entries<'a, T, I, V, K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Keys<'a, T, I, V, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> { self.entries.next().map(|(key, _)| key) }
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Keys<'a, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.entries.next_back().map(|(key, _)| key) }
}

/// Returned by [`Arena::iter`]
pub struct Iter<'a, T, V: Version> {
    slots: Occupied<core::slice::Iter<'a, Slot<T, V>>>,
}

impl<'a, T, V: Version> Iterator for Iter<'a, T, V> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|slot| unsafe { &*slot.data.value }) }
}

impl<T, V: Version> DoubleEndedIterator for Iter<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(|slot| unsafe { &*slot.data.value }) }
}

/// Returned by [`Arena::iter_mut`]
pub struct IterMut<'a, T, V: Version> {
    slots: Occupied<core::slice::IterMut<'a, Slot<T, V>>>,
}

impl<'a, T, V: Version> Iterator for IterMut<'a, T, V> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|slot| unsafe { &mut *slot.data.value }) }
}

impl<T, V: Version> DoubleEndedIterator for IterMut<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|slot| unsafe { &mut *slot.data.value })
    }
}

/// Returned by [`Arena::into_iter`]
pub struct IntoIter<T, V: Version> {
    slots: Occupied<std::vec::IntoIter<Slot<T, V>>>,
}

impl<T, V: Version> Iterator for IntoIter<T, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.next().map(|slot| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }
}

impl<T, V: Version> DoubleEndedIterator for IntoIter<T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|slot| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }
}

/// Returned by [`Arena::drain`]
pub struct Drain<'a, T, V: Version> {
    slots: Occupied<core::iter::Enumerate<core::slice::IterMut<'a, Slot<T, V>>>>,
    next: &'a mut usize,
    num_elements: &'a mut usize,
}

impl<T, V: Version> Drop for Drain<'_, T, V> {
    fn drop(&mut self) { self.for_each(drop); }
}

impl<'a, T, V: Version> Iterator for Drain<'a, T, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let next = &mut *self.next;
        let num_elements = &mut *self.num_elements;
        self.slots.next().map(|(index, slot)| unsafe {
            *num_elements -= 1;
            slot.remove_unchecked(index, next)
        })
    }
}

impl<T, V: Version> DoubleEndedIterator for Drain<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let next = &mut *self.next;
        let num_elements = &mut *self.num_elements;
        self.slots.next_back().map(|(index, slot)| unsafe {
            *num_elements -= 1;
            slot.remove_unchecked(index, next)
        })
    }
}

/// Returned by [`Arena::drain_filter`]
pub struct DrainFilter<'a, T, V: Version, F: FnMut(&mut T) -> bool> {
    slots: Occupied<core::iter::Enumerate<core::slice::IterMut<'a, Slot<T, V>>>>,
    next: &'a mut usize,
    num_elements: &'a mut usize,
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
        let filter = &mut self.filter;
        let panicked = &mut self.panicked;
        let (index, slot) = self
            .slots
            .try_fold((), |(), (index, slot)| {
                let panicked = crate::SetOnDrop(panicked);
                let return_value = filter(unsafe { &mut *slot.data.value });
                panicked.defuse();
                if return_value {
                    Err((index, slot))
                } else {
                    Ok(())
                }
            })
            .err()?;
        *self.num_elements -= 1;
        Some(unsafe { slot.remove_unchecked(index, self.next) })
    }
}

impl<T, V: Version, F: FnMut(&mut T) -> bool> DoubleEndedIterator for DrainFilter<'_, T, V, F> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let filter = &mut self.filter;
        let (index, slot) = self
            .slots
            .try_rfold((), |(), (index, slot)| {
                if filter(unsafe { &mut *slot.data.value }) {
                    Err((index, slot))
                } else {
                    Ok(())
                }
            })
            .err()?;
        *self.num_elements -= 1;
        Some(unsafe { slot.remove_unchecked(index, self.next) })
    }
}

/// Returned by [`Arena::entries`]
pub struct Entries<'a, T, I, V: Version, K> {
    slots: Occupied<core::iter::Enumerate<core::slice::Iter<'a, Slot<T, V>>>>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Entries<'a, T, I, V, K> {
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next()
            .map(|(index, slot)| unsafe { (K::new_unchecked(index, slot.version.save(), ident), &*slot.data.value) })
    }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Entries<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next_back()
            .map(|(index, slot)| unsafe { (K::new_unchecked(index, slot.version.save(), ident), &*slot.data.value) })
    }
}

/// Returned by [`Arena::entries_mut`]
pub struct EntriesMut<'a, T, I, V: Version, K> {
    slots: Occupied<core::iter::Enumerate<core::slice::IterMut<'a, Slot<T, V>>>>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for EntriesMut<'a, T, I, V, K> {
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next().map(|(index, slot)| unsafe {
            (
                K::new_unchecked(index, slot.version.save(), ident),
                &mut *slot.data.value,
            )
        })
    }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for EntriesMut<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next_back().map(|(index, slot)| unsafe {
            (
                K::new_unchecked(index, slot.version.save(), ident),
                &mut *slot.data.value,
            )
        })
    }
}

/// Returned by [`Arena::into_entries`]
pub struct IntoEntries<T, I, V: Version, K> {
    slots: Occupied<core::iter::Enumerate<std::vec::IntoIter<Slot<T, V>>>>,
    ident: I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for IntoEntries<T, I, V, K> {
    type Item = (K, T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next().map(|(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut slot.data.value);
            (K::new_unchecked(index, slot.version.save(), ident), value)
        })
    }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for IntoEntries<T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next_back().map(|(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut slot.data.value);
            (K::new_unchecked(index, slot.version.save(), ident), value)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn basic() {
        let mut arena = Arena::new();

        let a: usize = arena.insert(0);
        assert_eq!(a, 0);
        assert_eq!(arena[a], 0);
        assert_eq!(arena.remove(a), 0);
        assert_eq!(arena.get(a), None);

        let b: usize = arena.insert(10);
        assert_eq!(b, 0);
        assert_eq!(arena[b], 10);

        let b: usize = arena.insert(20);
        assert_eq!(b, 1);
        assert_eq!(arena[b], 20);
        assert_eq!(arena.remove(a), 10);
        assert_eq!(arena.get(a), None);

        let c: usize = arena.insert(30);
        assert_eq!(c, 0);
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
