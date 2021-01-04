use core::{
    mem::MaybeUninit,
    ops::{Index, IndexMut},
};

use std::{boxed::Box, vec::Vec};

use crate::sparse::{Arena as SparseArena, ArenaAccess, BuildArenaKey, VacantEntry as SparseVacantEntry};

pub struct Arena<T, I = ()> {
    slots: SparseArena<usize, I>,
    keys: Box<[MaybeUninit<usize>]>,
    values: Vec<T>,
}

pub struct VacantEntry<'a, T, K> {
    sparse: SparseVacantEntry<'a, usize, K>,
    values: &'a mut Vec<T>,
    keys: &'a mut MaybeUninit<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self { Self::with_ident(()) }
}

impl<T, I> Arena<T, I> {
    pub fn with_ident(ident: I) -> Self {
        Self {
            slots: SparseArena::with_ident(ident),
            values: Vec::new(),
            keys: Box::new([]),
        }
    }

    pub fn ident(&self) -> &I { self.slots.ident() }

    pub fn len(&self) -> usize { self.values.len() }

    pub fn capacity(&self) -> usize { self.values.capacity() }

    pub fn reserve(&mut self, additional: usize) {
        let len = self.values.len();

        self.values.reserve(additional);
        let mut keys = Vec::from(core::mem::take(&mut self.keys));
        keys.reserve(additional);
        unsafe {
            let cap = keys.capacity();
            keys.set_len(cap);
        }
        self.keys = keys.into();

        if let Some(additional) = (self.slots.capacity() - len).checked_sub(additional) {
            self.slots.reserve(additional);
        }
    }

    #[cold]
    #[inline(never)]
    fn reserve_cold(&mut self, additional: usize) { self.reserve(additional) }
}

impl<'a, T, I> VacantEntry<'a, T, I> {
    pub fn key<K: BuildArenaKey<I>>(&self) -> K { self.sparse.key() }

    pub fn insert<K: BuildArenaKey<I>>(self, value: T) -> K {
        unsafe {
            let index = self.values.len();
            self.values.as_mut_ptr().add(index).write(value);
            self.values.set_len(index + 1);
            let key = self.sparse.insert(index);
            *self.keys = MaybeUninit::new(index);
            key
        }
    }
}

impl<T, I> Arena<T, I> {
    pub fn parse_key<K: BuildArenaKey<I>>(&self, index: usize) -> Option<K> { self.slots.parse_key(index) }
}

impl<T, I> Arena<T, I> {
    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I> {
        let len = self.len();

        if len == self.capacity() {
            self.reserve_cold(1);
        }

        VacantEntry {
            sparse: self.slots.vacant_entry(),
            values: &mut self.values,
            keys: unsafe { self.keys.get_unchecked_mut(len) },
        }
    }

    pub fn insert<K: BuildArenaKey<I>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    pub fn remove<K: ArenaAccess<I>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove<K: ArenaAccess<I>>(&mut self, key: K) -> Option<T> {
        let index = self.slots.try_remove(key)?;
        let value = self.values.swap_remove(index);

        let keys = self.keys.as_mut_ptr();

        let end = unsafe {
            let this = keys.add(index);
            let end = keys.add(self.values.len());
            this.swap(end);
            *this.cast::<usize>()
        };

        let end = unsafe { crate::sparse::TrustedIndex::new(end) };

        // if the last element wasn't removed
        if let Some(end) = self.slots.get_mut(end) {
            *end = index
        }

        Some(value)
    }

    pub fn contains<K: ArenaAccess<I>>(&self, key: K) -> bool { self.slots.contains(key) }

    pub fn get<K: ArenaAccess<I>>(&self, key: K) -> Option<&T> {
        let &slot = self.slots.get(key)?;
        unsafe { Some(self.values.get_unchecked(slot)) }
    }

    pub fn get_mut<K: ArenaAccess<I>>(&mut self, key: K) -> Option<&mut T> {
        let &slot = self.slots.get(key)?;
        unsafe { Some(self.values.get_unchecked_mut(slot)) }
    }

    pub fn values(&self) -> core::slice::Iter<'_, T> { self.values.iter() }

    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, T> { self.values.iter_mut() }

    pub fn into_values(self) -> std::vec::IntoIter<T> { self.values.into_iter() }

    pub fn keys<'a, K: 'a + BuildArenaKey<I>>(&'a self) -> impl 'a + ExactSizeIterator<Item = K> {
        unsafe { keys(&self.keys, self.values.len(), &self.slots) }
    }

    pub fn entries<'a, K: 'a + BuildArenaKey<I>>(&'a self) -> impl 'a + ExactSizeIterator<Item = (K, &T)> {
        let mut keys = unsafe { keys(&self.keys, self.values.len(), &self.slots) };
        self.values.iter().map(move |value| {
            let key = match keys.next() {
                Some(key) => key,
                None => unsafe { core::hint::unreachable_unchecked() },
            };

            (key, value)
        })
    }

    pub fn entries_mut<'a, K: 'a + BuildArenaKey<I>>(&'a mut self) -> impl 'a + ExactSizeIterator<Item = (K, &mut T)> {
        let mut keys = unsafe { keys(&self.keys, self.values.len(), &self.slots) };
        self.values.iter_mut().map(move |value| {
            let key = match keys.next() {
                Some(key) => key,
                None => unsafe { core::hint::unreachable_unchecked() },
            };

            (key, value)
        })
    }

    pub fn into_entries<K: BuildArenaKey<I>>(self) -> impl ExactSizeIterator<Item = (K, T)> {
        let mut keys = Vec::from(self.keys).into_iter();
        let slots = self.slots;
        self.values.into_iter().map(move |value| {
            let key = match keys.next() {
                Some(key) => key,
                None => unsafe { core::hint::unreachable_unchecked() },
            };

            let key = match slots.parse_key(unsafe { key.assume_init() }) {
                Some(key) => key,
                None => unsafe { core::hint::unreachable_unchecked() },
            };

            (key, value)
        })
    }
}

unsafe fn keys<'a, I, K: BuildArenaKey<I>>(
    keys: &'a [MaybeUninit<usize>],
    len: usize,
    slots: &'a SparseArena<usize, I>,
) -> impl 'a + ExactSizeIterator<Item = K> {
    let keys = keys.get_unchecked(..len);
    let keys = core::slice::from_raw_parts(keys.as_ptr().cast::<usize>(), keys.len());
    keys.iter().map(move |&index| match slots.parse_key(index) {
        Some(index) => index,
        None => core::hint::unreachable_unchecked(),
    })
}

impl<T, I, K: ArenaAccess<I>> Index<K> for Arena<T, I> {
    type Output = T;

    fn index(&self, key: K) -> &Self::Output { self.get(key).expect("Tried to access `Arena` with a stale `Key`") }
}

impl<T, I, K: ArenaAccess<I>> IndexMut<K> for Arena<T, I> {
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

impl<T, I> Extend<T> for Arena<T, I> {
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        iter.for_each(move |value| drop::<usize>(self.vacant_entry().insert(value)));
    }
}

use std::fmt;

impl<T: fmt::Debug, I: fmt::Debug> fmt::Debug for Arena<T, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("slots", &self.slots)
            .field("values", &self.values)
            .finish()
    }
}
