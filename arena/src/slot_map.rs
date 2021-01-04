macro_rules! imp_slot_map {
    (
        new $($const:ident)?: $new:expr,
        slots: $slots:ident
    ) => {
        #[derive(Debug, Clone)]
        #[repr(transparent)]
        pub struct SlotMap<T>(pub Arena<T, ()>);

        pub struct VacantEntry<'a, T>(pub imp::VacantEntry<'a, T, ()>);

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

            pub fn insert(&mut self, value: T) -> Key<usize> { self.0.insert(value) }

            pub fn parse_key(&self, index: usize) -> Option<Key<usize>> { self.0.parse_key(index) }

            pub fn get(&mut self, index: usize) -> Option<&T> { self.0.get(index) }

            pub fn get_mut(&mut self, index: usize) -> Option<&mut T> { self.0.get_mut(index) }

            pub fn remove(&mut self, index: usize) -> T { self.0.remove(index) }

            pub fn try_remove(&mut self, index: usize) -> Option<T> { self.0.try_remove(index) }

            pub fn iter(&self) -> Values<'_, T> { self.0.values() }

            pub fn iter_mut(&mut self) -> ValuesMut<'_, T> { self.0.values_mut() }

            pub fn entries(&self) -> imp::Entries<'_, T, (), DefaultVersion, usize> { self.0.entries() }

            pub fn entries_mut(&mut self) -> imp::EntriesMut<'_, T, (), DefaultVersion, usize> { self.0.entries_mut() }

            pub fn into_entries(self) -> imp::IntoEntries<T, (), DefaultVersion, usize> { self.0.into_entries() }
        }

        impl<T> IntoIterator for SlotMap<T> {
            type IntoIter = IntoValues<T>;
            type Item = T;

            fn into_iter(self) -> Self::IntoIter { self.0.into_values() }
        }

        impl<T> Index<usize> for SlotMap<T> {
            type Output = T;

            fn index(&self, index: usize) -> &Self::Output { &self.0[index] }
        }

        impl<T> IndexMut<usize> for SlotMap<T> {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output { &mut self.0[index] }
        }
    };
}

pub mod dense;
pub mod hop;
pub mod sparse;
