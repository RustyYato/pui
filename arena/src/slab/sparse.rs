use core::ops::{Index, IndexMut};

use crate::{
    sparse::{self, Arena},
    version::Unversioned,
};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Slab<T>(pub Arena<T, (), Unversioned>);

pub struct VacantEntry<'a, T>(pub sparse::VacantEntry<'a, T, (), Unversioned>);

impl<T> VacantEntry<'_, T> {
    pub fn key(&self) -> usize { self.0.key() }

    pub fn insert(self, value: T) -> usize { self.0.insert(value) }
}

impl<T> Default for Slab<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Slab<T> {
    pub const fn new() -> Self { Self(Arena::INIT) }

    pub fn slots(&self) -> usize { self.0.slots() }

    pub fn capacity(&self) -> usize { self.0.capacity() }

    pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }

    pub fn clear(&mut self) { self.0.clear(); }

    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T> { VacantEntry(self.0.vacant_entry()) }

    pub fn insert(&mut self, value: T) -> usize { self.0.insert(value) }

    pub fn contains(&self, index: usize) -> bool { self.0.parse_key::<usize>(index).is_some() }

    pub fn get(&mut self, index: usize) -> Option<&T> { self.0.get(index) }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> { self.0.get_mut(index) }

    pub fn remove(&mut self, index: usize) -> T { self.0.remove(index) }

    pub fn try_remove(&mut self, index: usize) -> Option<T> { self.0.try_remove(index) }

    pub fn iter(&self) -> sparse::Values<'_, T, Unversioned> { self.0.values() }

    pub fn iter_mut(&mut self) -> sparse::ValuesMut<'_, T, Unversioned> { self.0.values_mut() }

    pub fn entries(&self) -> sparse::Entries<'_, T, (), Unversioned, usize> { self.0.entries() }

    pub fn entries_mut(&mut self) -> sparse::EntriesMut<'_, T, (), Unversioned, usize> { self.0.entries_mut() }

    pub fn into_entries(self) -> sparse::IntoEntries<T, (), Unversioned, usize> { self.0.into_entries() }
}

impl<T> IntoIterator for Slab<T> {
    type IntoIter = sparse::IntoValues<T, Unversioned>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter { self.0.into_values() }
}

impl<T> Index<usize> for Slab<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output { &self.0[index] }
}

impl<T> IndexMut<usize> for Slab<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output { &mut self.0[index] }
}
