macro_rules! imp_scoped {
    (
        @forward
        slots: $slots:ident
        ($($keys:ident)?)
        ($($version:ident)?)
    ) => {
        /// The key for [`Arena`]
        pub type Key<'scope, V = crate::version::DefaultVersion> = key::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, <V as crate::version::Version>::Save>;

        /// The backing arena type
        pub type BaseArena<'scope, T, V = crate::version::DefaultVersion> = imp::Arena<T, pui_core::scoped::Scoped<'scope>, V>;
        /// The backing vacant entry type
        pub type BaseVacantEntry<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::VacantEntry<'a, T, pui_core::scoped::Scoped<'scope>, V>;

        /// A scoped arena
        pub struct Arena<'scope, T, V: crate::version::Version = crate::version::DefaultVersion>(pub imp::Arena<T, pui_core::scoped::Scoped<'scope>, V>);
        /// a vacant entry into [`Arena`]
        pub struct VacantEntry<'a, 'scope, T, V: crate::version::Version = crate::version::DefaultVersion>(pub imp::VacantEntry<'a, T, pui_core::scoped::Scoped<'scope>, V>);

        /// Returned from [`Arena::entries`]
        pub type Entries<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Entries<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;
        /// Returned from [`Arena::entries_mut`]
        pub type EntriesMut<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::EntriesMut<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;
        /// Returned from [`Arena::into_entries`]
        pub type IntoEntries<'scope, T, V = crate::version::DefaultVersion> = imp::IntoEntries<T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

        impl<'scope, T, V: crate::version::Version> VacantEntry<'_, 'scope, T, V> {
            /// see [`VacantEntry::key`](imp::VacantEntry::key)
            pub fn key(&self) -> Key<'scope, V> { self.0.key() }

            /// see [`VacantEntry::insert`](imp::VacantEntry::insert)
            pub fn insert(self, value: T) -> Key<'scope, V> { self.0.insert(value) }
        }

        impl<'scope, T, V: crate::version::Version> Arena<'scope, T, V> {
            /// Create a new arena
            pub fn new(ident: pui_core::scoped::Scoped<'scope>) -> Self {
                Self(BaseArena::with_ident(ident))
            }
            /// see [`Arena::ident`](imp::Arena::ident)
            pub fn ident(&self) -> &pui_core::scoped::Scoped<'scope> { self.0.ident() }
            /// see [`Arena::is_empty`](imp::Arena::is_empty)
            pub fn is_empty(&self) -> bool { self.0.is_empty() }
            /// see [`Arena::len`](imp::Arena::is_empty)
            pub fn len(&self) -> usize { self.0.len() }
            /// see [`Arena::capacity`](imp::Arena::capacity)
            pub fn capacity(&self) -> usize { self.0.capacity() }
            /// see [`Arena::reserve`](imp::Arena::reserve)
            pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
            /// see [`Arena::parse_key`](imp::Arena::parse_key)
            pub fn parse_key(&self, index: usize) -> Option<Key<'scope, V>> { self.0.parse_key(index) }
            /// see [`Arena::vacant_entry`](imp::Arena::vacant_entry)
            pub fn vacant_entry(&mut self) -> VacantEntry<'_, 'scope, T, V> { VacantEntry(self.0.vacant_entry()) }
            /// see [`Arena::insert`](imp::Arena::insert)
            pub fn insert(&mut self, value: T) -> Key<'scope, V> { self.0.insert(value) }
            /// see [`Arena::contains`](imp::Arena::contains)
            pub fn contains(&self, key: Key<'scope, V>) -> bool { self.0.contains(key) }
            /// see [`Arena::remove`](imp::Arena::remove)
            pub fn remove(&mut self, key: Key<'scope, V>) -> T { self.0.remove(key) }
            /// see [`Arena::try_remove`](imp::Arena::try_remove)
            pub fn try_remove(&mut self, key: Key<'scope, V>) -> Option<T> { self.0.try_remove(key) }
            /// see [`Arena::delete`](imp::Arena::delete)
            pub fn delete(&mut self, key: Key<'scope, V>) -> bool { self.0.delete(key) }
            /// see [`Arena::get`](imp::Arena::get)
            pub fn get(&self, key: Key<'scope, V>) -> Option<&T> { self.0.get(key) }
            /// see [`Arena::get_mut`](imp::Arena::get_mut)
            pub fn get_mut(&mut self, key: Key<'scope, V>) -> Option<&mut T> { self.0.get_mut(key) }
            /// see [`Arena::get_unchecked`](imp::Arena::get_unchecked)
            #[allow(clippy::missing_safety_doc)]
            pub unsafe fn get_unchecked(&self, index: usize) -> &T { self.0.get_unchecked(index) }
            /// see [`Arena::get_unchecked_mut`](imp::Arena::get_unchecked_mut)
            #[allow(clippy::missing_safety_doc)]
            pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T { self.0.get_unchecked_mut(index) }
            /// see [`Arena::delete_all`](imp::Arena::delete_all)
            pub fn delete_all(&mut self) { self.0.delete_all() }
            /// see [`Arena::retain`](imp::Arena::retain)
            pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, f: F) { self.0.retain(f) }
            /// see [`Arena::keys`](imp::Arena::keys)
            pub fn keys(&self) -> Keys<'_, 'scope $(, $keys)?, V> { self.0.keys() }
            /// see [`Arena::iter`](imp::Arena::iter)
            pub fn iter(&self) -> Iter<'_, T $(, $version)?> { self.0.iter() }
            /// see [`Arena::iter_mut`](imp::Arena::iter_mut)
            pub fn iter_mut(&mut self) -> IterMut<'_, T $(, $version)?> { self.0.iter_mut() }
            /// see [`Arena::drain`](imp::Arena::drain)
            pub fn drain(&mut self) -> Drain<'_, 'scope, T, V> { self.0.drain() }
            /// see [`Arena::drain_filter`](imp::Arena::drain_filter)
            pub fn drain_filter<F: FnMut(&mut T) -> bool>(&mut self, filter: F) -> DrainFilter<'_, 'scope, T, F, V> { self.0.drain_filter(filter) }
            /// see [`Arena::entries`](imp::Arena::entries)
            pub fn entries(&self) -> Entries<'_, 'scope, T, V> { self.0.entries() }
            /// see [`Arena::entries_mut`](imp::Arena::entries_mut)
            pub fn entries_mut(&mut self) -> EntriesMut<'_, 'scope, T, V> { self.0.entries_mut() }
            /// see [`Arena::into_entries`](imp::Arena::into_entries)
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
        /// a sparse scoped arena
        ///
        /// see [base::sparse](crate::base::sparse) for details
        pub mod sparse {
            use core::ops::*;
            use crate::base::sparse as imp;
            use crate::base::sparse as key;

            /// Returned from [`Arena::iter`]
            pub type Iter<'a, T, V = crate::version::DefaultVersion> = imp::Iter<'a, T, V>;
            /// Returned from [`Arena::iter_mut`]
            pub type IterMut<'a, T, V = crate::version::DefaultVersion> = imp::IterMut<'a, T, V>;
            /// Returned from [`Arena::into_iter`]
            pub type IntoIter<T, V = crate::version::DefaultVersion> = imp::IntoIter<T, V>;

            /// Returned from [`Arena::drain`]
            pub type Drain<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Drain<'a, T, V>;
            /// Returned from [`Arena::drain_filter`]
            pub type DrainFilter<'a, 'scope, T, F, V = crate::version::DefaultVersion> = imp::DrainFilter<'a, T, V, F>;

            /// Returned from [`Arena::keys`]
            pub type Keys<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Keys<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

            imp_scoped! {
                @forward
                slots: slots
                (T)
                (V)
            }
        }

        /// a hop scoped arena
        ///
        /// see [base::sparse](crate::base::sparse) for details
        pub mod hop {
            use core::ops::*;
            use crate::base::hop as imp;
            use crate::base::hop as key;

            /// Returned from [`Arena::iter`]
            pub type Iter<'a, T, V = crate::version::DefaultVersion> = imp::Iter<'a, T, V>;
            /// Returned from [`Arena::iter_mut`]
            pub type IterMut<'a, T, V = crate::version::DefaultVersion> = imp::IterMut<'a, T, V>;
            /// Returned from [`Arena::into_iter`]
            pub type IntoIter<T, V = crate::version::DefaultVersion> = imp::IntoIter<T, V>;

            /// Returned from [`Arena::drain`]
            pub type Drain<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Drain<'a, T, V>;
            /// Returned from [`Arena::drain_filter`]
            pub type DrainFilter<'a, 'scope, T, F, V = crate::version::DefaultVersion> = imp::DrainFilter<'a, T, V, F>;

            /// Returned from [`Arena::keys`]
            pub type Keys<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Keys<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

            imp_scoped! {
                @forward
                slots: len
                (T)
                (V)
            }
        }

        /// a dense scoped arena
        ///
        /// see [base::sparse](crate::base::sparse) for details
        pub mod dense {
            use core::ops::*;
            use crate::base::dense as imp;
            use crate::base::sparse as key;

            /// Returned from [`Arena::iter`]
            pub type Iter<'a, T> = core::slice::Iter<'a, T>;
            /// Returned from [`Arena::iter_mut`]
            pub type IterMut<'a, T> = core::slice::IterMut<'a, T>;
            /// Returned from [`Arena::into_iter`]
            pub type IntoIter<T> = std::vec::IntoIter<T>;

            /// Returned from [`Arena::drain`]
            pub type Drain<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Drain<'a, T, pui_core::scoped::Scoped<'scope>, V>;
            /// Returned from [`Arena::drain_filter`]
            pub type DrainFilter<'a, 'scope, T, F, V = crate::version::DefaultVersion> = imp::DrainFilter<'a, T, pui_core::scoped::Scoped<'scope>, V, F>;

            /// Returned from [`Arena::keys`]
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
