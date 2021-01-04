use core::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Index, IndexMut},
};

use std::{boxed::Box, vec::Vec};

use crate::{
    base::sparse::{Arena as SparseArena, ArenaAccess, BuildArenaKey, VacantEntry as SparseVacantEntry},
    version::{DefaultVersion, Version},
};

#[derive(Clone)]
pub struct Arena<T, I = (), V: Version = DefaultVersion> {
    slots: SparseArena<usize, I, V>,
    keys: Box<[MaybeUninit<usize>]>,
    values: Vec<T>,
}

pub struct VacantEntry<'a, T, K, V: Version = DefaultVersion> {
    sparse: SparseVacantEntry<'a, usize, K, V>,
    values: &'a mut Vec<T>,
    keys: &'a mut MaybeUninit<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self { Self::with_ident(()) }
}

impl<T, V: Version> Arena<T, (), V> {
    pub fn clear(&mut self) {
        self.slots.clear();
        self.values.clear();
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
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

impl<'a, T, I, V: Version> VacantEntry<'a, T, I, V> {
    pub fn key<K: BuildArenaKey<I, V>>(&self) -> K { self.sparse.key() }

    pub fn insert<K: BuildArenaKey<I, V>>(self, value: T) -> K {
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

impl<T, I, V: Version> Arena<T, I, V> {
    pub fn parse_key<K: BuildArenaKey<I, V>>(&self, index: usize) -> Option<K> { self.slots.parse_key(index) }
}

impl<T, I, V: Version> Arena<T, I, V> {
    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I, V> {
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

    pub fn insert<K: BuildArenaKey<I, V>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    pub fn remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<T> {
        let index = self.slots.try_remove(key)?;
        let value = self.values.swap_remove(index);

        let keys = self.keys.as_mut_ptr();

        let end = unsafe {
            let this = keys.add(index);
            let end = keys.add(self.values.len());
            this.swap(end);
            *this.cast::<usize>()
        };

        let end = unsafe { crate::base::sparse::TrustedIndex::new(end) };

        // if the last element wasn't removed
        if let Some(end) = self.slots.get_mut(end) {
            *end = index
        }

        Some(value)
    }

    pub fn contains<K: ArenaAccess<I, V>>(&self, key: K) -> bool { self.slots.contains(key) }

    pub fn get<K: ArenaAccess<I, V>>(&self, key: K) -> Option<&T> {
        let &slot = self.slots.get(key)?;
        unsafe { Some(self.values.get_unchecked(slot)) }
    }

    pub fn get_mut<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<&mut T> {
        let &slot = self.slots.get(key)?;
        unsafe { Some(self.values.get_unchecked_mut(slot)) }
    }

    pub fn remove_all(&mut self) {
        self.slots.remove_all();
        self.values.clear();
    }

    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        for i in (0..self.values.len()).rev() {
            let value = unsafe { self.values.get_unchecked_mut(i) };

            if !f(value) {
                self.try_remove(i);
            }
        }
    }

    pub fn values(&self) -> core::slice::Iter<'_, T> { self.values.iter() }

    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, T> { self.values.iter_mut() }

    pub fn into_values(self) -> std::vec::IntoIter<T> { self.values.into_iter() }

    pub fn keys<'a, K: 'a + BuildArenaKey<I, V>>(&'a self) -> Keys<'_, I, V, K> {
        unsafe { keys(&self.keys, self.values.len(), &self.slots) }
    }

    pub fn into_keys<'a, K: 'a + BuildArenaKey<I, V>>(self) -> IntoKeys<I, V, K> {
        unsafe { into_keys(self.keys, self.values.len(), self.slots) }
    }

    pub fn entries<'a, K: 'a + BuildArenaKey<I, V>>(&'a self) -> Entries<'_, T, I, V, K> {
        Entries {
            keys: unsafe { keys(&self.keys, self.values.len(), &self.slots) },
            values: self.values.iter(),
        }
    }

    pub fn entries_mut<'a, K: 'a + BuildArenaKey<I, V>>(&'a mut self) -> EntriesMut<'_, T, I, V, K> {
        EntriesMut {
            keys: unsafe { keys(&self.keys, self.values.len(), &self.slots) },
            values: self.values.iter_mut(),
        }
    }

    pub fn into_entries<K: BuildArenaKey<I, V>>(self) -> IntoEntries<T, I, V, K> {
        IntoEntries {
            keys: unsafe { into_keys(self.keys, self.values.len(), self.slots) },
            values: self.values.into_iter(),
        }
    }
}

unsafe fn keys<'a, I, V: Version, K: BuildArenaKey<I, V>>(
    keys: &'a [MaybeUninit<usize>],
    len: usize,
    slots: &'a SparseArena<usize, I, V>,
) -> Keys<'a, I, V, K> {
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
    len: usize,
    slots: SparseArena<usize, I, V>,
) -> IntoKeys<I, V, K> {
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

impl<T, I, V: Version, K: ArenaAccess<I, V>> Index<K> for Arena<T, I, V> {
    type Output = T;

    fn index(&self, key: K) -> &Self::Output { self.get(key).expect("Tried to access `Arena` with a stale `Key`") }
}

impl<T, I, V: Version, K: ArenaAccess<I, V>> IndexMut<K> for Arena<T, I, V> {
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

impl<T, I, V: Version> Extend<T> for Arena<T, I, V> {
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        iter.for_each(move |value| drop::<usize>(self.vacant_entry().insert(value)));
    }
}

use std::fmt;

impl<T: fmt::Debug, I: fmt::Debug, V: Version + fmt::Debug> fmt::Debug for Arena<T, I, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("slots", &self.slots)
            .field("values", &self.values)
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

pub struct IntoKeys<I, V: Version, K> {
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

macro_rules! entry_impl {
    () => {
        fn next(&mut self) -> Option<Self::Item> {
            self.keys.next().map(move |key| {
                let value = self
                    .values
                    .next()
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });
                (key, value)
            })
        }

        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            self.keys.nth(n).map(move |key| {
                let value = self
                    .values
                    .nth(n)
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });
                (key, value)
            })
        }

        fn size_hint(&self) -> (usize, Option<usize>) { self.keys.size_hint() }
    };
    (rev) => {
        fn next_back(&mut self) -> Option<Self::Item> {
            self.keys.next_back().map(move |key| {
                let value = self
                    .values
                    .next_back()
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });
                (key, value)
            })
        }

        fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
            self.keys.nth_back(n).map(move |key| {
                let value = self
                    .values
                    .nth_back(n)
                    .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });
                (key, value)
            })
        }
    };
}

pub struct Entries<'a, T, I, V: Version, K> {
    values: core::slice::Iter<'a, T>,
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

pub struct EntriesMut<'a, T, I, V: Version, K> {
    values: core::slice::IterMut<'a, T>,
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

pub struct IntoEntries<T, I, V: Version, K> {
    values: std::vec::IntoIter<T>,
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
