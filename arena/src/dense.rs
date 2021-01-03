use core::{
    mem::MaybeUninit,
    ops::{Index, IndexMut},
};

use std::{boxed::Box, vec::Vec};

use pui_core::OneShotIdentifier;

use crate::sparse::{Arena as SparseArena, Key as SparseKey, VacantEntry as SparseVacantEntry};

pub struct Key<T>(SparseKey<T>);

pub struct Arena<T, I> {
    slots: SparseArena<usize, I>,
    keys: Box<[MaybeUninit<usize>]>,
    values: Vec<T>,
}

pub struct VacantEntry<'a, T, I: OneShotIdentifier> {
    sparse: SparseVacantEntry<'a, usize, I>,
    values: &'a mut Vec<T>,
    keys: &'a mut MaybeUninit<usize>,
}

impl<T, I> Arena<T, I> {
    pub fn new(ident: I) -> Self {
        Self {
            slots: SparseArena::new(ident),
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

impl<'a, T, I: OneShotIdentifier> VacantEntry<'a, T, I> {
    pub fn key(&self) -> Key<I::Token> { Key(self.sparse.key()) }

    pub fn insert(self, value: T) -> Key<I::Token> {
        unsafe {
            let index = self.values.len();
            self.values.as_mut_ptr().add(index).write(value);
            self.values.set_len(index + 1);
            let key = self.sparse.insert(index);
            *self.keys = MaybeUninit::new(key.index().get());
            Key(key)
        }
    }
}

impl<T, I: OneShotIdentifier> Arena<T, I> {
    pub fn parse_index(&self, index: usize) -> Option<Key<I::Token>> { self.slots.parse_index(index).map(Key) }

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

    pub fn insert(&mut self, value: T) -> Key<I::Token> { self.vacant_entry().insert(value) }

    pub fn remove(&mut self, key: Key<I::Token>) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove(&mut self, key: Key<I::Token>) -> Option<T> {
        let index = self.slots.try_remove(key.0)?;
        let value = self.values.swap_remove(index);

        let keys = self.keys.as_mut_ptr();

        let end = unsafe {
            let this = keys.add(index);
            let end = keys.add(self.values.len());
            this.swap(end);
            *this.cast::<usize>()
        };

        if self.slots.slots() < end {
            unsafe { core::hint::unreachable_unchecked() }
        }

        // if the last element wasn't removed
        if let Some(end) = self.slots.parse_index(end) {
            self.slots[end] = index
        }

        Some(value)
    }

    pub fn contains(&self, key: &Key<I::Token>) -> bool { self.slots.contains(&key.0) }

    pub fn get(&self, key: Key<I::Token>) -> Option<&T> {
        let &slot = self.slots.get(key.0)?;
        unsafe { Some(self.values.get_unchecked(slot)) }
    }

    pub fn get_mut(&mut self, key: Key<I::Token>) -> Option<&mut T> {
        let &slot = self.slots.get(key.0)?;
        unsafe { Some(self.values.get_unchecked_mut(slot)) }
    }

    pub fn values(&self) -> core::slice::Iter<'_, T> { self.values.iter() }

    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, T> { self.values.iter_mut() }

    pub fn into_values(self) -> std::vec::IntoIter<T> { self.values.into_iter() }

    pub fn keys(&self) -> impl '_ + ExactSizeIterator<Item = Key<I::Token>> {
        unsafe { keys(&self.keys, self.values.len(), &self.slots) }
    }

    pub fn entries(&self) -> impl '_ + ExactSizeIterator<Item = (Key<I::Token>, &T)> {
        let mut keys = unsafe { keys(&self.keys, self.values.len(), &self.slots) };
        self.values.iter().map(move |value| {
            let key = match keys.next() {
                Some(key) => key,
                None => unsafe { core::hint::unreachable_unchecked() },
            };

            (key, value)
        })
    }

    pub fn entries_mut(&mut self) -> impl '_ + ExactSizeIterator<Item = (Key<I::Token>, &mut T)> {
        let mut keys = unsafe { keys(&self.keys, self.values.len(), &self.slots) };
        self.values.iter_mut().map(move |value| {
            let key = match keys.next() {
                Some(key) => key,
                None => unsafe { core::hint::unreachable_unchecked() },
            };

            (key, value)
        })
    }
}

unsafe fn keys<'a, I: OneShotIdentifier>(
    keys: &'a [MaybeUninit<usize>],
    len: usize,
    slots: &'a SparseArena<usize, I>,
) -> impl 'a + ExactSizeIterator<Item = Key<I::Token>> {
    let keys = keys.get_unchecked(..len);
    let keys = core::slice::from_raw_parts(keys.as_ptr().cast::<usize>(), keys.len());
    keys.iter().map(move |&index| match slots.parse_index(index) {
        Some(index) => Key(index),
        None => core::hint::unreachable_unchecked(),
    })
}

impl<T, I: OneShotIdentifier> Index<Key<I::Token>> for Arena<T, I> {
    type Output = T;

    fn index(&self, key: Key<I::Token>) -> &Self::Output {
        self.get(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

impl<T, I: OneShotIdentifier> IndexMut<Key<I::Token>> for Arena<T, I> {
    fn index_mut(&mut self, key: Key<I::Token>) -> &mut Self::Output {
        self.get_mut(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

impl<T, I: OneShotIdentifier> Extend<T> for Arena<T, I> {
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        iter.for_each(move |value| drop(self.insert(value)));
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
