macro_rules! imp_slot_map {
    (
        new $($const:ident)?: $new:expr,
        slots: $slots:ident
    ) => {
        #[derive(Debug, Clone)]
        #[repr(transparent)]
        pub struct SlotMap<T>(pub Arena<T, ()>);

        pub struct VacantEntry<'a, T>(pub imp::VacantEntry<'a, T, ()>);

        pub type Key = key::Key<usize>;

        pub type Entries<'a, T> = imp::Entries<'a, T, (), DefaultVersion, usize>;
        pub type EntriesMut<'a, T> = imp::EntriesMut<'a, T, (), DefaultVersion, usize>;
        pub type IntoEntries<T> = imp::IntoEntries<T, (), DefaultVersion, usize>;

        impl<T> VacantEntry<'_, T> {
            pub fn key(&self) -> usize { self.0.key() }

            pub fn insert(self, value: T) -> usize { self.0.insert(value) }
        }

        impl<T> Default for SlotMap<T> {
            fn default() -> Self { Self::new() }
        }

        impl<T> SlotMap<T> {
            pub $($const)? fn new() -> Self { Self($new) }
            pub fn $slots(&self) -> usize { self.0.$slots() }
            pub fn capacity(&self) -> usize { self.0.capacity() }
            pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
            pub fn clear(&mut self) { self.0.clear(); }
            pub fn vacant_entry(&mut self) -> VacantEntry<'_, T> { VacantEntry(self.0.vacant_entry()) }
            pub fn insert(&mut self, value: T) -> Key { self.0.insert(value) }
            pub fn parse_key(&self, index: usize) -> Option<Key> { self.0.parse_key(index) }
            pub fn get(&mut self, key: Key) -> Option<&T> { self.0.get(key) }
            pub fn get_mut(&mut self, key: Key) -> Option<&mut T> { self.0.get_mut(key) }
            pub unsafe fn get_unchecked(&mut self, index: usize) -> &T { self.0.get_unchecked(index) }
            pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T { self.0.get_unchecked_mut(index) }
            pub fn remove(&mut self, key: Key) -> T { self.0.remove(key) }
            pub fn try_remove(&mut self, key: Key) -> Option<T> { self.0.try_remove(key) }
            pub fn delete(&mut self, key: Key) -> bool { self.0.delete(key) }
            pub fn delete_all(&mut self) { self.0.delete_all() }
            pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, f: F) { self.0.retain(f) }
            pub fn iter(&self) -> Iter<'_, T> { self.0.iter() }
            pub fn iter_mut(&mut self) -> IterMut<'_, T> { self.0.iter_mut() }
            pub fn drain(&mut self) -> Drain<'_, T> { self.0.drain() }
            pub fn drain_filter<F: FnMut(&mut T) -> bool>(&mut self, filter: F) -> DrainFilter<'_, T, F> { self.0.drain_filter(filter) }
            pub fn entries(&self) -> Entries<'_, T> { self.0.entries() }
            pub fn entries_mut(&mut self) -> EntriesMut<'_, T> { self.0.entries_mut() }
            pub fn into_entries(self) -> IntoEntries<T> { self.0.into_entries() }
        }

        impl<T> IntoIterator for SlotMap<T> {
            type IntoIter = IntoIter<T>;
            type Item = T;

            fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
        }

        impl<T> Index<Key> for SlotMap<T> {
            type Output = T;

            fn index(&self, key: Key) -> &Self::Output { &self.0[key] }
        }

        impl<T> IndexMut<Key> for SlotMap<T> {
            fn index_mut(&mut self, key: Key) -> &mut Self::Output { &mut self.0[key] }
        }
    };
}

pub mod dense;
pub mod hop;
pub mod sparse;
