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

            pub fn get(&mut self, index: Key) -> Option<&T> { self.0.get(index) }

            pub fn get_mut(&mut self, index: Key) -> Option<&mut T> { self.0.get_mut(index) }

            pub fn remove(&mut self, index: Key) -> T { self.0.remove(index) }

            pub fn try_remove(&mut self, index: Key) -> Option<T> { self.0.try_remove(index) }

            pub fn iter(&self) -> Values<'_, T> { self.0.values() }

            pub fn iter_mut(&mut self) -> ValuesMut<'_, T> { self.0.values_mut() }

            pub fn entries(&self) -> Entries<'_, T> { self.0.entries() }

            pub fn entries_mut(&mut self) -> EntriesMut<'_, T> { self.0.entries_mut() }

            pub fn into_entries(self) -> IntoEntries<T> { self.0.into_entries() }
        }

        impl<T> IntoIterator for SlotMap<T> {
            type IntoIter = IntoValues<T>;
            type Item = T;

            fn into_iter(self) -> Self::IntoIter { self.0.into_values() }
        }

        impl<T> Index<Key> for SlotMap<T> {
            type Output = T;

            fn index(&self, index: Key) -> &Self::Output { &self.0[index] }
        }

        impl<T> IndexMut<Key> for SlotMap<T> {
            fn index_mut(&mut self, index: Key) -> &mut Self::Output { &mut self.0[index] }
        }
    };
}

pub mod dense;
pub mod hop;
pub mod sparse;
