macro_rules! imp_scoped {
    (
        @forward
        slots: $slots:ident
        ($($keys:ident)?)
        ($($version:ident)?)
    ) => {
        pub type Key<'scope, V = crate::version::DefaultVersion> = key::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, <V as crate::version::Version>::Save>;

        pub type BaseArena<'scope, T, V = crate::version::DefaultVersion> = imp::Arena<T, pui_core::scoped::Scoped<'scope>, V>;
        pub type BaseVacantEntry<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::VacantEntry<'a, T, pui_core::scoped::Scoped<'scope>, V>;

        pub struct Arena<'scope, T, V: crate::version::Version = crate::version::DefaultVersion>(pub imp::Arena<T, pui_core::scoped::Scoped<'scope>, V>);
        pub struct VacantEntry<'a, 'scope, T, V: crate::version::Version = crate::version::DefaultVersion>(pub imp::VacantEntry<'a, T, pui_core::scoped::Scoped<'scope>, V>);

        pub type Entries<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Entries<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;
        pub type EntriesMut<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::EntriesMut<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;
        pub type IntoEntries<'scope, T, V = crate::version::DefaultVersion> = imp::IntoEntries<T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

        impl<'scope, T, V: crate::version::Version> VacantEntry<'_, 'scope, T, V> {
            pub fn key(&self) -> Key<'scope, V> { self.0.key() }

            pub fn insert(self, value: T) -> Key<'scope, V> { self.0.insert(value) }
        }

        impl<'scope, T, V: crate::version::Version> Arena<'scope, T, V> {
            pub fn new(ident: pui_core::scoped::Scoped<'scope>) -> Self {
                Self(BaseArena::with_ident(ident))
            }

            pub fn ident(&self) -> &pui_core::scoped::Scoped<'scope> { self.0.ident() }
            pub fn $slots(&self) -> usize { self.0.$slots() }
            pub fn capacity(&self) -> usize { self.0.capacity() }
            pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
            pub fn parse_key(&self, index: usize) -> Option<Key<'scope, V>> { self.0.parse_key(index) }
            pub fn vacant_entry(&mut self) -> VacantEntry<'_, 'scope, T, V> { VacantEntry(self.0.vacant_entry()) }
            pub fn insert(&mut self, value: T) -> Key<'scope, V> { self.0.insert(value) }
            pub fn contains(&self, key: Key<'scope, V>) -> bool { self.0.contains(key) }
            pub fn remove(&mut self, key: Key<'scope, V>) -> T { self.0.remove(key) }
            pub fn try_remove(&mut self, key: Key<'scope, V>) -> Option<T> { self.0.try_remove(key) }
            pub fn delete(&mut self, key: Key<'scope, V>) -> bool { self.0.delete(key) }
            pub fn get(&self, key: Key<'scope, V>) -> Option<&T> { self.0.get(key) }
            pub fn get_mut(&mut self, key: Key<'scope, V>) -> Option<&mut T> { self.0.get_mut(key) }
            pub unsafe fn get_unchecked(&self, index: usize) -> &T { self.0.get_unchecked(index) }
            pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T { self.0.get_unchecked_mut(index) }
            pub fn delete_all(&mut self) { self.0.delete_all() }
            pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, f: F) { self.0.retain(f) }
            pub fn keys(&self) -> Keys<'_, 'scope $(, $keys)?, V> { self.0.keys() }
            pub fn iter(&self) -> Iter<'_, T $(, $version)?> { self.0.iter() }
            pub fn iter_mut(&mut self) -> IterMut<'_, T $(, $version)?> { self.0.iter_mut() }
            pub fn entries(&self) -> Entries<'_, 'scope, T, V> { self.0.entries() }
            pub fn entries_mut(&mut self) -> EntriesMut<'_, 'scope, T, V> { self.0.entries_mut() }
            pub fn into_entries(self) -> IntoEntries<'scope, T, V> { self.0.into_entries() }
        }

        impl<T, V: crate::version::Version> core::iter::IntoIterator for Arena<'_, T, V> {
            type IntoIter = IntoIter<T $(, $version)?>;
            type Item = T;

            fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
        }

        impl<'scope, T, V: crate::version::Version> Index<Key<'scope, V>> for Arena<'scope, T, V> {
            type Output = T;

            fn index(&self, index: Key<'scope, V>) -> &Self::Output { &self.0[index] }
        }

        impl<'scope, T, V: crate::version::Version> IndexMut<Key<'scope, V>> for Arena<'scope, T, V> {
            fn index_mut(&mut self, index: Key<'scope, V>) -> &mut Self::Output { &mut self.0[index] }
        }
    };
    (@build_module) => {
        pub mod sparse {
            use core::ops::*;
            use crate::base::sparse as imp;
            use crate::base::sparse as key;

            pub type Iter<'a, T, V = crate::version::DefaultVersion> = imp::Iter<'a, T, V>;
            pub type IterMut<'a, T, V = crate::version::DefaultVersion> = imp::IterMut<'a, T, V>;
            pub type IntoIter<T, V = crate::version::DefaultVersion> = imp::IntoIter<T, V>;

            pub type Keys<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Keys<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

            imp_scoped! {
                @forward
                slots: slots
                (T)
                (V)
            }
        }

        pub mod hop {
            use core::ops::*;
            use crate::base::hop as imp;
            use crate::base::hop as key;

            pub type Iter<'a, T, V = crate::version::DefaultVersion> = imp::Iter<'a, T, V>;
            pub type IterMut<'a, T, V = crate::version::DefaultVersion> = imp::IterMut<'a, T, V>;
            pub type IntoIter<T, V = crate::version::DefaultVersion> = imp::IntoIter<T, V>;

            pub type Keys<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Keys<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

            imp_scoped! {
                @forward
                slots: len
                (T)
                (V)
            }
        }

        pub mod dense {
            use core::ops::*;
            use crate::base::dense as imp;
            use crate::base::sparse as key;

            pub type Iter<'a, T> = core::slice::Iter<'a, T>;
            pub type IterMut<'a, T> = core::slice::IterMut<'a, T>;
            pub type IntoIter<T> = std::vec::IntoIter<T>;

            pub type Keys<'a, 'scope, V = crate::version::DefaultVersion> = imp::Keys<'a, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

            imp_scoped! {
                @forward
                slots: len
                ()
                ()
            }
        }
    };
}

imp_scoped! { @build_module }
