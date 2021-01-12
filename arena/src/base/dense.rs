//! Dense Arenas - Slow Access, Super-Fast Iteration, Slow Mutation, Large memory footprint
//!
//! A dense arena stores all of it's values in a `Vec<_>`,and stores the
//! slots-state seprately. However, this requries a double indirection to access
//! values, so it will be slower to acces. The benefit to this layout however, is
//! faster iteration. Iterating over the values is just iterating over a slice.
//! So it's orders of magnitude faster than either hop arenas or sparse arenas.
//!
//! The other downside to dense arenas, is they require 3 separate allocations,
//! one for the slots, one for the values, and one to convert indicies in the
//! values allocation to indicies in the slots allocation.
//!
//! Each slot is versioned by using [`Version`] trait. See [`Version`] for docs
//! on version exhaustion. Once a slot's version exhausts, it will not be reused.

use core::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Index, IndexMut},
};

use std::{boxed::Box, vec::Vec};

use crate::{
    base::sparse::{Arena as SparseArena, VacantEntry as SparseVacantEntry},
    version::{DefaultVersion, Version},
    ArenaAccess, BuildArenaKey,
};

/// A dense arena
pub struct Arena<T, I = (), V: Version = DefaultVersion> {
    slots: SparseArena<usize, I, V>,
    keys: Box<[MaybeUninit<usize>]>,
    values: Box<[MaybeUninit<T>]>,
}

/// An empty slot in a dense arena
pub struct VacantEntry<'a, T, K, V: Version = DefaultVersion> {
    slots: SparseVacantEntry<'a, usize, K, V>,
    value: &'a mut MaybeUninit<T>,
    keys: &'a mut MaybeUninit<usize>,
    index: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Arena<T> {
    /// Create a new key from an id and version
    pub fn new() -> Self { Self::with_ident(()) }
}

impl<T: Clone, I: Clone, V: Version> Clone for Arena<T, I, V> {
    fn clone(&self) -> Self {
        let slots = self.slots.clone();
        let keys = unsafe { self.keys.get_unchecked(..slots.len()) };
        let keys = unsafe { &*(keys as *const [MaybeUninit<usize>] as *const [usize]) };
        let keys = keys.to_vec().into_boxed_slice();
        let keys = unsafe { Box::from_raw(Box::into_raw(keys) as *mut [MaybeUninit<usize>]) };
        let values = unsafe { self.values.get_unchecked(..slots.len()) };
        let values = unsafe { &*(values as *const [MaybeUninit<T>] as *const [T]) };
        let values = values.to_vec().into_boxed_slice();
        let values = unsafe { Box::from_raw(Box::into_raw(values) as *mut [MaybeUninit<T>]) };
        Self { slots, keys, values }
    }
}

impl<T, V: Version> Arena<T, (), V> {
    /// Clear the arena without reducing it's capacity
    pub fn clear(&mut self) {
        let len = self.slots.len();
        self.slots.clear();
        unsafe { core::ptr::drop_in_place(self.values.get_unchecked_mut(..len) as *mut [MaybeUninit<T>] as *mut [T]) }
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    /// Create a new arena with the given identifier
    pub fn with_ident(ident: I) -> Self {
        Self {
            slots: SparseArena::with_ident(ident),
            values: Box::new([]),
            keys: Box::new([]),
        }
    }

    /// Get the associated identifier for this arena
    pub fn ident(&self) -> &I { self.slots.ident() }

    /// Returns true if the arena is empty
    pub fn is_empty(&self) -> bool { self.slots.is_empty() }

    /// Returns the number of elements in this arena
    pub fn len(&self) -> usize { self.slots.len() }

    /// Returns the capacity of this arena
    pub fn capacity(&self) -> usize { self.values.len() }

    /// Reserves capacity for at least additional more elements to be inserted
    /// in the given Arena<T>. The collection may reserve more space to avoid
    /// frequent reallocations. After calling reserve, capacity will be greater
    /// than or equal to self.len() + additional. Does nothing if capacity is
    /// already sufficient.
    pub fn reserve(&mut self, additional: usize) {
        fn reserve_box<T>(bx: &mut Box<[MaybeUninit<T>]>, additional: usize) {
            let mut vec = Vec::from(core::mem::take(bx));
            vec.reserve(additional);
            unsafe {
                let cap = vec.capacity();
                vec.set_len(cap);
            }
            *bx = vec.into();
        }

        struct Abort;

        impl Drop for Abort {
            fn drop(&mut self) { panic!() }
        }

        let abort_on_panic = Abort;

        reserve_box(&mut self.values, additional);
        reserve_box(&mut self.keys, additional);

        core::mem::forget(abort_on_panic);

        self.slots.reserve(additional);
    }

    #[cold]
    #[inline(never)]
    fn reserve_cold(&mut self, additional: usize) { self.reserve(additional) }
}

impl<'a, T, I, V: Version> VacantEntry<'a, T, I, V> {
    /// Get the key associated with the `VacantEntry`, this key can be used
    /// once this `VacantEntry` gets filled
    pub fn key<K: BuildArenaKey<I, V>>(&self) -> K { self.slots.key() }

    /// Insert an element into the vacant entry
    pub fn insert<K: BuildArenaKey<I, V>>(self, value: T) -> K {
        *self.value = MaybeUninit::new(value);
        let key: K = self.slots.insert(self.index);
        *self.keys = MaybeUninit::new(key.index());
        key
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    /// Check if an index is in bounds, and if it is return a `Key<_, _>` to it
    pub fn parse_key<K: BuildArenaKey<I, V>>(&self, index: usize) -> Option<K> { self.slots.parse_key(index) }
}

impl<T, I, V: Version> Arena<T, I, V> {
    /// Return a handle to a vacant entry allowing for further manipulation.
    ///
    /// This function is useful when creating values that must contain their
    /// key. The returned VacantEntry reserves a slot in the arena and is able
    /// to query the associated key.
    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I, V> {
        let len = self.len();

        if len == self.values.len() {
            self.reserve_cold(1);
        }

        VacantEntry {
            slots: self.slots.vacant_entry(),
            value: unsafe { self.values.get_unchecked_mut(len) },
            keys: unsafe { self.keys.get_unchecked_mut(len) },
            index: len,
        }
    }

    /// Insert a value in the arena, returning key assigned to the value.
    ///
    /// The returned key can later be used to retrieve or remove the value
    /// using indexed lookup and remove. Additional capacity is allocated
    /// if needed.
    pub fn insert<K: BuildArenaKey<I, V>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    /// Return true if a value is associated with the given key.
    pub fn contains<K: ArenaAccess<I, V>>(&self, key: K) -> bool { self.slots.contains(key) }

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
        let index = self.slots.try_remove(key)?;
        Some(self.remove_unchecked(index))
    }

    fn remove_unchecked(&mut self, index: usize) -> T {
        if index == self.slots.len() {
            unsafe { return self.values.get_unchecked(index).as_ptr().read() }
        }

        let value;

        unsafe {
            // remove element from vec
            let last = self.slots.len();
            let ptr = self.values.as_mut_ptr().cast::<T>();
            value = ptr.add(index).read();
            ptr.add(index).copy_from_nonoverlapping(ptr.add(last), 1);

            // remove back ref to slot
            let ptr = self.keys.as_mut_ptr();
            let back_ref = *ptr.add(last).cast::<usize>();
            ptr.add(index).copy_from_nonoverlapping(ptr.add(last), 1);

            *self.slots.get_unchecked_mut(back_ref) = index;
        }

        value
    }

    /// Removes the value associated with the given key.
    ///
    /// The key is then released and may be associated with future stored values,
    /// if the versioning strategy allows it.
    ///
    /// Returns true if the value was removed, an false otherwise
    pub fn delete<K: ArenaAccess<I, V>>(&mut self, key: K) -> bool {
        struct Fixup<'a, T, I, V: Version> {
            ptr: *mut T,
            index: usize,
            last: usize,
            keys: &'a mut [MaybeUninit<usize>],
            slots: &'a mut SparseArena<usize, I, V>,
        }

        impl<T, I, V: Version> Drop for Fixup<'_, T, I, V> {
            fn drop(&mut self) {
                unsafe {
                    let Self {
                        ptr,
                        index,
                        last,
                        ref mut keys,
                        ref mut slots,
                    } = *self;

                    ptr.add(index).copy_from_nonoverlapping(ptr.add(last), 1);

                    // remove back ref to slot
                    let ptr = keys.as_mut_ptr();
                    let back_ref = *ptr.add(last).cast::<usize>();
                    ptr.add(index).copy_from_nonoverlapping(ptr.add(last), 1);
                    *slots.get_unchecked_mut(back_ref) = index;
                }
            }
        }

        let index = match self.slots.try_remove(key) {
            Some(index) => index,
            None => return false,
        };

        unsafe {
            // remove element from vec
            let last = self.slots.len();
            let ptr = self.values.as_mut_ptr().cast::<T>();

            let _fixup = if index == last {
                None
            } else {
                Some(Fixup {
                    ptr,
                    index,
                    last,
                    keys: &mut self.keys,
                    slots: &mut self.slots,
                })
            };

            ptr.add(index).drop_in_place();

            true
        }
    }

    /// Return a shared reference to the value associated with the given key.
    ///
    /// If the given key is not associated with a value, then None is returned.
    pub fn get<K: ArenaAccess<I, V>>(&self, key: K) -> Option<&T> {
        let &slot = self.slots.get(key)?;
        unsafe { Some(&*self.values.get_unchecked(slot).as_ptr()) }
    }

    /// Return a unique reference to the value associated with the given key.
    ///
    /// If the given key is not associated with a value, then None is returned.
    pub fn get_mut<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<&mut T> {
        let &slot = self.slots.get(key)?;
        unsafe { Some(&mut *self.values.get_unchecked_mut(slot).as_mut_ptr()) }
    }

    /// Return a shared reference to the value associated with the
    /// given key without performing bounds checking, or checks
    /// if there is a value associated to the key
    ///
    /// # Safety
    ///
    /// `contains` should return true with the given index.
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        let &slot = self.slots.get_unchecked(index);
        &*self.values.get_unchecked(slot).as_ptr()
    }

    /// Return a unique reference to the value associated with the
    /// given key without performing bounds checking, or checks
    /// if there is a value associated to the key
    ///
    /// # Safety
    ///
    /// `contains` should return true with the given index.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        let &slot = self.slots.get_unchecked(index);
        &mut *self.values.get_unchecked_mut(slot).as_mut_ptr()
    }

    /// Deletes all elements from the arena
    pub fn delete_all(&mut self) {
        let len = self.slots.len();
        self.slots.delete_all();
        unsafe { core::ptr::drop_in_place(self.values.get_unchecked_mut(..len) as *mut [MaybeUninit<T>] as *mut [T]) }
    }

    /// Retain only the elements specified by the predicate.
    ///
    /// If the predicate returns for a given element true,
    /// then the element is kept in the arena.
    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        for i in (0..self.slots.len()).rev() {
            let value = unsafe { &mut *self.values.get_unchecked_mut(i).as_mut_ptr() };

            if !f(value) {
                self.delete(unsafe { crate::TrustedIndex::new(i) });
            }
        }
    }

    /// An iterator over the keys of the arena, in no particular order
    pub fn keys<'a, K: 'a + BuildArenaKey<I, V>>(&'a self) -> Keys<'_, I, V, K> {
        unsafe { keys(&self.keys, &self.slots) }
    }

    /// An iterator of shared references to values of the arena,
    /// in no particular order
    pub fn iter(&self) -> core::slice::Iter<'_, T> { unsafe { iter(&self.values, self.slots.len()) } }

    /// An iterator of unique references to values of the arena,
    /// in no particular order
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        unsafe { iter_mut(&mut self.values, self.slots.len()) }
    }

    /// Return a draining iterator that removes all elements from the
    /// arena and yields the removed items.
    ///
    /// Note: Elements are removed even if the iterator is only partially
    /// consumed or not consumed at all.
    pub fn drain(&mut self) -> Drain<'_, T, I, V> {
        Drain {
            range: 0..self.slots.len(),
            arena: self,
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
    pub fn drain_filter<F: FnMut(&mut T) -> bool>(&mut self, filter: F) -> DrainFilter<'_, T, I, V, F> {
        DrainFilter {
            range: 0..self.slots.len(),
            arena: self,
            filter,
            panicked: false,
        }
    }

    /// An iterator of keys and shared references to values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn entries<'a, K: 'a + BuildArenaKey<I, V>>(&'a self) -> Entries<'_, T, I, V, K> {
        Entries {
            iter: unsafe { iter(&self.values, self.slots.len()) },
            keys: unsafe { keys(&self.keys, &self.slots) },
        }
    }

    /// An iterator of keys and unique references to values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn entries_mut<'a, K: 'a + BuildArenaKey<I, V>>(&'a mut self) -> EntriesMut<'_, T, I, V, K> {
        EntriesMut {
            iter: unsafe { iter_mut(&mut self.values, self.slots.len()) },
            keys: unsafe { keys(&self.keys, &self.slots) },
        }
    }

    /// An iterator of keys and values of the arena,
    /// in no particular order, with each key being associated
    /// to the corrosponding value
    pub fn into_entries<K: BuildArenaKey<I, V>>(self) -> IntoEntries<T, I, V, K> {
        IntoEntries {
            iter: unsafe { into_iter(self.values, self.slots.len()) },
            keys: unsafe { into_keys(self.keys, self.slots) },
        }
    }
}

unsafe fn iter<T>(slice: &[MaybeUninit<T>], len: usize) -> core::slice::Iter<'_, T> {
    let ptr = slice as *const [MaybeUninit<T>] as *const T;
    core::slice::from_raw_parts(ptr, len).iter()
}

unsafe fn iter_mut<T>(slice: &mut [MaybeUninit<T>], len: usize) -> core::slice::IterMut<'_, T> {
    let ptr = slice as *mut [MaybeUninit<T>] as *mut T;
    core::slice::from_raw_parts_mut(ptr, len).iter_mut()
}

unsafe fn into_iter<T>(slice: Box<[MaybeUninit<T>]>, len: usize) -> std::vec::IntoIter<T> {
    let cap = slice.len();
    let ptr = Box::into_raw(slice) as *mut [MaybeUninit<T>] as *mut T;
    let vec = Vec::from_raw_parts(ptr, len, cap);
    vec.into_iter()
}

unsafe fn keys<'a, I, V: Version, K: BuildArenaKey<I, V>>(
    keys: &'a [MaybeUninit<usize>],
    slots: &'a SparseArena<usize, I, V>,
) -> Keys<'a, I, V, K> {
    let len = slots.len();
    let keys = keys.get_unchecked(..len);
    let keys = core::slice::from_raw_parts(keys.as_ptr().cast::<usize>(), keys.len());

    Keys {
        keys: keys.iter().copied(),
        slots,
        key: PhantomData,
    }
}

unsafe fn into_keys<I, V: Version, K: BuildArenaKey<I, V>>(
    keys: Box<[MaybeUninit<usize>]>,
    slots: SparseArena<usize, I, V>,
) -> IntoKeys<I, V, K> {
    let len = slots.len();
    let mut keys = ManuallyDrop::new(keys);
    let cap = keys.len();
    let keys = keys.as_mut_ptr().cast::<usize>();
    let keys = std::vec::Vec::from_raw_parts(keys, len, cap);

    IntoKeys {
        keys: keys.into_iter(),
        slots,
        key: PhantomData,
    }
}

impl<T, I, V: Version> IntoIterator for Arena<T, I, V> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter { unsafe { into_iter(self.values, self.slots.len()) } }
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

use std::fmt;

impl<T: fmt::Debug, I: fmt::Debug, V: Version + fmt::Debug> fmt::Debug for Arena<T, I, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys = unsafe {
            let keys = self.keys.get_unchecked(..self.slots.len());
            core::slice::from_raw_parts(keys.as_ptr().cast::<usize>(), keys.len())
        };
        let values = unsafe {
            let values = self.values.get_unchecked(..self.slots.len());
            core::slice::from_raw_parts(values.as_ptr().cast::<T>(), values.len())
        };
        f.debug_struct("Arena")
            .field("slots", &self.slots)
            .field("values", &values)
            .field("keys", &keys)
            .finish()
    }
}

macro_rules! keys_impl {
    () => {
        type Item = K;

        fn next(&mut self) -> Option<Self::Item> {
            self.keys.next().map(move |index| {
                self.slots
                    .parse_key(index)
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() })
            })
        }

        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            self.keys.nth(n).map(move |index| {
                self.slots
                    .parse_key(index)
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() })
            })
        }

        fn size_hint(&self) -> (usize, Option<usize>) { self.keys.size_hint() }
    };
    (rev) => {
        fn next_back(&mut self) -> Option<Self::Item> {
            self.keys.next_back().map(move |index| {
                self.slots
                    .parse_key(index)
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() })
            })
        }

        fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
            self.keys.nth_back(n).map(move |index| {
                self.slots
                    .parse_key(index)
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() })
            })
        }
    };
}

/// Returned by [`Arena::keys`]
pub struct Keys<'a, I, V: Version, K> {
    keys: core::iter::Copied<core::slice::Iter<'a, usize>>,
    slots: &'a SparseArena<usize, I, V>,
    key: PhantomData<fn() -> K>,
}

impl<'a, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Keys<'a, I, V, K> {
    keys_impl! {}
}

impl<I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Keys<'_, I, V, K> {
    keys_impl! { rev }
}

impl<I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for Keys<'_, I, V, K> {}
impl<I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Keys<'_, I, V, K> {}

struct IntoKeys<I, V: Version, K> {
    keys: std::vec::IntoIter<usize>,
    slots: SparseArena<usize, I, V>,
    key: PhantomData<fn() -> K>,
}

impl<'a, I, V: Version, K: BuildArenaKey<I, V>> Iterator for IntoKeys<I, V, K> {
    keys_impl! {}
}

impl<I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for IntoKeys<I, V, K> {
    keys_impl! { rev }
}

impl<I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for IntoKeys<I, V, K> {}
impl<I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for IntoKeys<I, V, K> {}

/// Returned by [`Arena::drain`]
pub struct Drain<'a, T, I, V: Version> {
    arena: &'a mut Arena<T, I, V>,
    range: core::ops::Range<usize>,
}

impl<T, I, V: Version> Drop for Drain<'_, T, I, V> {
    fn drop(&mut self) { self.for_each(drop); }
}

impl<'a, T, I, V: Version> Iterator for Drain<'a, T, I, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.range.next()?;
        unsafe {
            let key = self.arena.keys.get_unchecked(index).as_ptr().read();
            self.arena.slots.delete_unchecked(key);
            Some(self.arena.remove_unchecked(index))
        }
    }
}

impl<T, I, V: Version> DoubleEndedIterator for Drain<'_, T, I, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let index = self.range.next_back()?;
        unsafe {
            let key = self.arena.keys.get_unchecked(index).as_ptr().read();
            self.arena.slots.delete_unchecked(key);
            Some(self.arena.remove_unchecked(index))
        }
    }
}

/// Returned by [`Arena::drain_filter`]
pub struct DrainFilter<'a, T, I, V: Version, F: FnMut(&mut T) -> bool> {
    arena: &'a mut Arena<T, I, V>,
    range: core::ops::Range<usize>,
    filter: F,
    panicked: bool,
}

impl<T, I, V: Version, F: FnMut(&mut T) -> bool> Drop for DrainFilter<'_, T, I, V, F> {
    fn drop(&mut self) {
        if !self.panicked {
            self.for_each(drop);
        }
    }
}

impl<'a, T, I, V: Version, F: FnMut(&mut T) -> bool> Iterator for DrainFilter<'a, T, I, V, F> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.range.next()?;
            unsafe {
                let panicked = crate::SetOnDrop(&mut self.panicked);
                let value = &mut *self.arena.values.get_unchecked_mut(index).as_mut_ptr();
                let do_filter = (self.filter)(value);
                panicked.defuse();
                if do_filter {
                    let key = self.arena.keys.get_unchecked(index).as_ptr().read();
                    self.arena.slots.delete_unchecked(key);
                    return Some(self.arena.remove_unchecked(index))
                }
            }
        }
    }
}

impl<T, I, V: Version, F: FnMut(&mut T) -> bool> DoubleEndedIterator for DrainFilter<'_, T, I, V, F> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.range.next_back()?;
            unsafe {
                let value = &mut *self.arena.values.get_unchecked_mut(index).as_mut_ptr();
                if (self.filter)(value) {
                    let key = self.arena.keys.get_unchecked(index).as_ptr().read();
                    self.arena.slots.delete_unchecked(key);
                    return Some(self.arena.remove_unchecked(index))
                }
            }
        }
    }
}

macro_rules! entry_impl {
    () => {
        fn next(&mut self) -> Option<Self::Item> {
            self.keys.next().map(move |key| {
                let value = match self.iter.next() {
                    Some(item) => item,
                    None => unsafe { core::hint::unreachable_unchecked() },
                };
                (key, value)
            })
        }

        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            self.keys.nth(n).map(move |key| {
                let value = match self.iter.nth(n) {
                    Some(item) => item,
                    None => unsafe { core::hint::unreachable_unchecked() },
                };
                (key, value)
            })
        }

        fn size_hint(&self) -> (usize, Option<usize>) { self.keys.size_hint() }
    };
    (rev) => {
        fn next_back(&mut self) -> Option<Self::Item> {
            self.keys.next_back().map(move |key| {
                let value = match self.iter.next_back() {
                    Some(item) => item,
                    None => unsafe { core::hint::unreachable_unchecked() },
                };
                (key, value)
            })
        }

        fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
            self.keys.nth_back(n).map(move |key| {
                let value = match self.iter.nth_back(n) {
                    Some(item) => item,
                    None => unsafe { core::hint::unreachable_unchecked() },
                };
                (key, value)
            })
        }
    };
}

/// Returned by [`Arena::entries`]
pub struct Entries<'a, T, I, V: Version, K> {
    iter: core::slice::Iter<'a, T>,
    keys: Keys<'a, I, V, K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Entries<'a, T, I, V, K> {
    type Item = (K, &'a T);

    entry_impl! {}
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Entries<'_, T, I, V, K> {
    entry_impl! { rev }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for Entries<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Entries<'_, T, I, V, K> {}

/// Returned by [`Arena::entries_mut`]
pub struct EntriesMut<'a, T, I, V: Version, K> {
    iter: core::slice::IterMut<'a, T>,
    keys: Keys<'a, I, V, K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for EntriesMut<'a, T, I, V, K> {
    type Item = (K, &'a mut T);

    entry_impl! {}
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for EntriesMut<'_, T, I, V, K> {
    entry_impl! { rev }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for EntriesMut<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for EntriesMut<'_, T, I, V, K> {}

/// Returned by [`Arena::into_entries`]
pub struct IntoEntries<T, I, V: Version, K> {
    iter: std::vec::IntoIter<T>,
    keys: IntoKeys<I, V, K>,
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for IntoEntries<T, I, V, K> {
    type Item = (K, T);

    entry_impl! {}
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for IntoEntries<T, I, V, K> {
    entry_impl! { rev }
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
        assert_eq!(arena[b], 20);
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
