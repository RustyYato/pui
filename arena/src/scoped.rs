//! Scoped arenas in terms of the generic arenas in [`base`](crate::base)
//!
//! These arenas can only be used in the scope they are declared in.
//!
//! This allows for purely compile-time bounds checks, using the
//! [`pui_core::scoped`] module and Rust's borrow checker!
//!
//! The module structure here is identical to [`crate::base`], and
//! you can look there for detailed documentation about the types.
//! Each implementation of `SlotMap` will have all the methods from the
//! corrosponding `Arena`, and those that take or produce generic keys
//! will instead take/produce a `Key`.
//!
//! In each module, you'll find an `ScopedArena` newtype (with one public field),
//! a `ScopedVacantEntry` newtype (again with one public field). These are thin
//! wrappers around their generic counterparts. Their only serve the purpose
//! of making error messages easier to parse, and use a default `Key`.
//! You will also find a vareity of type aliases for various iterators, and
//! for the default `Key` type for ease of use.
//!
//! If you want to access the raw backing `Arena`/`VacantEntry`, you still can,
//! it is the only public field of each scoped arena/vacant entry.

use core::borrow::{Borrow, BorrowMut};

use crate::{version::Version, ArenaKey, BuildArenaKey, CompleteValidator, Validator};

macro_rules! imp_scoped {
    (
        @forward
        slots: $slots:ident
        ($($keys:ident)?)
        ($($version:ident)?)
    ) => {
        /// The key for [`ScopedArena`]
        pub type Key<'scope, V = crate::version::DefaultVersion> = super::ScopedKey<'scope, <V as crate::version::Version>::Save>;

        /// The backing arena type
        pub type BaseArena<'scope, T, V = crate::version::DefaultVersion> = imp::Arena<T, pui_core::scoped::Scoped<'scope>, V>;
        /// The backing vacant entry type
        pub type BaseVacantEntry<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::VacantEntry<'a, T, pui_core::scoped::Scoped<'scope>, V>;

        /// A scoped arena
        pub struct ScopedArena<'scope, T, V: crate::version::Version = crate::version::DefaultVersion>(pub imp::Arena<T, pui_core::scoped::Scoped<'scope>, V>);
        /// a vacant entry into [`ScopedArena`]
        pub struct ScopedVacantEntry<'a, 'scope, T, V: crate::version::Version = crate::version::DefaultVersion>(pub imp::VacantEntry<'a, T, pui_core::scoped::Scoped<'scope>, V>);

        /// Returned from [`ScopedArena::entries`]
        pub type Entries<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Entries<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;
        /// Returned from [`ScopedArena::entries_mut`]
        pub type EntriesMut<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::EntriesMut<'a, T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;
        /// Returned from [`ScopedArena::into_entries`]
        pub type IntoEntries<'scope, T, V = crate::version::DefaultVersion> = imp::IntoEntries<T, pui_core::scoped::Scoped<'scope>, V, Key<'scope, V>>;

        impl<'scope, T, V: crate::version::Version> ScopedVacantEntry<'_, 'scope, T, V> {
            /// see [`VacantEntry::key`](imp::VacantEntry::key)
            pub fn key(&self) -> Key<'scope, V> { self.0.key() }

            /// see [`VacantEntry::insert`](imp::VacantEntry::insert)
            pub fn insert(self, value: T) -> Key<'scope, V> { self.0.insert(value) }
        }

        impl<'scope, T, V: crate::version::Version> ScopedArena<'scope, T, V> {
            /// Create a new arena
            pub fn new(ident: pui_core::scoped::Scoped<'scope>) -> Self {
                Self(BaseArena::with_ident(ident))
            }
            /// see [`ScopedArena::ident`](imp::Arena::ident)
            pub fn ident(&self) -> &pui_core::scoped::Scoped<'scope> { self.0.ident() }
            /// see [`ScopedArena::is_empty`](imp::Arena::is_empty)
            pub fn is_empty(&self) -> bool { self.0.is_empty() }
            /// see [`ScopedArena::len`](imp::Arena::is_empty)
            pub fn len(&self) -> usize { self.0.len() }
            /// see [`ScopedArena::capacity`](imp::Arena::capacity)
            pub fn capacity(&self) -> usize { self.0.capacity() }
            /// see [`ScopedArena::reserve`](imp::Arena::reserve)
            pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
            /// see [`ScopedArena::parse_key`](imp::Arena::parse_key)
            pub fn parse_key(&self, index: usize) -> Option<Key<'scope, V>> { self.0.parse_key(index) }
            /// see [`ScopedArena::vacant_entry`](imp::Arena::vacant_entry)
            pub fn vacant_entry(&mut self) -> ScopedVacantEntry<'_, 'scope, T, V> { ScopedVacantEntry(self.0.vacant_entry()) }
            /// see [`ScopedArena::insert`](imp::Arena::insert)
            pub fn insert(&mut self, value: T) -> Key<'scope, V> { self.0.insert(value) }
            /// see [`ScopedArena::contains`](imp::Arena::contains)
            pub fn contains(&self, key: Key<'scope, V>) -> bool { self.0.contains(key) }
            /// see [`ScopedArena::remove`](imp::Arena::remove)
            pub fn remove(&mut self, key: Key<'scope, V>) -> T { self.0.remove(key) }
            /// see [`ScopedArena::try_remove`](imp::Arena::try_remove)
            pub fn try_remove(&mut self, key: Key<'scope, V>) -> Option<T> { self.0.try_remove(key) }
            /// see [`ScopedArena::delete`](imp::Arena::delete)
            pub fn delete(&mut self, key: Key<'scope, V>) -> bool { self.0.delete(key) }
            /// see [`ScopedArena::get`](imp::Arena::get)
            pub fn get(&self, key: Key<'scope, V>) -> Option<&T> { self.0.get(key) }
            /// see [`ScopedArena::get_mut`](imp::Arena::get_mut)
            pub fn get_mut(&mut self, key: Key<'scope, V>) -> Option<&mut T> { self.0.get_mut(key) }
            /// see [`ScopedArena::get_unchecked`](imp::Arena::get_unchecked)
            #[allow(clippy::missing_safety_doc)]
            pub unsafe fn get_unchecked(&self, index: usize) -> &T { self.0.get_unchecked(index) }
            /// see [`ScopedArena::get_unchecked_mut`](imp::Arena::get_unchecked_mut)
            #[allow(clippy::missing_safety_doc)]
            pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T { self.0.get_unchecked_mut(index) }
            /// see [`ScopedArena::delete_all`](imp::Arena::delete_all)
            pub fn delete_all(&mut self) { self.0.delete_all() }
            /// see [`ScopedArena::retain`](imp::Arena::retain)
            pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, f: F) { self.0.retain(f) }
            /// see [`ScopedArena::keys`](imp::Arena::keys)
            pub fn keys(&self) -> Keys<'_, 'scope $(, $keys)?, V> { self.0.keys() }
            /// see [`ScopedArena::iter`](imp::Arena::iter)
            pub fn iter(&self) -> Iter<'_, T $(, $version)?> { self.0.iter() }
            /// see [`ScopedArena::iter_mut`](imp::Arena::iter_mut)
            pub fn iter_mut(&mut self) -> IterMut<'_, T $(, $version)?> { self.0.iter_mut() }
            /// see [`ScopedArena::drain`](imp::Arena::drain)
            pub fn drain(&mut self) -> Drain<'_, 'scope, T, V> { self.0.drain() }
            /// see [`ScopedArena::drain_filter`](imp::Arena::drain_filter)
            pub fn drain_filter<F: FnMut(&mut T) -> bool>(&mut self, filter: F) -> DrainFilter<'_, 'scope, T, F, V> { self.0.drain_filter(filter) }
            /// see [`ScopedArena::entries`](imp::Arena::entries)
            pub fn entries(&self) -> Entries<'_, 'scope, T, V> { self.0.entries() }
            /// see [`ScopedArena::entries_mut`](imp::Arena::entries_mut)
            pub fn entries_mut(&mut self) -> EntriesMut<'_, 'scope, T, V> { self.0.entries_mut() }
            /// see [`ScopedArena::into_entries`](imp::Arena::into_entries)
            pub fn into_entries(self) -> IntoEntries<'scope, T, V> { self.0.into_entries() }
        }

        impl<T, V: crate::version::Version> core::iter::IntoIterator for ScopedArena<'_, T, V> {
            type IntoIter = IntoIter<T $(, $version)?>;
            type Item = T;

            fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
        }

        impl<'scope, T, V: crate::version::Version> Index<Key<'scope, V>> for ScopedArena<'scope, T, V> {
            type Output = T;

            fn index(&self, index: Key<'scope, V>) -> &Self::Output { &self.0[index] }
        }

        impl<'scope, T, V: crate::version::Version> IndexMut<Key<'scope, V>> for ScopedArena<'scope, T, V> {
            fn index_mut(&mut self, index: Key<'scope, V>) -> &mut Self::Output { &mut self.0[index] }
        }
    };
    (@build_module) => {
        /// a sparse scoped arena
        ///
        /// see [`base::sparse`](crate::base::sparse) for details
        pub mod sparse {
            use core::ops::*;
            use crate::base::sparse as imp;

            /// Returned from [`ScopedArena::iter`]
            pub type Iter<'a, T, V = crate::version::DefaultVersion> = imp::Iter<'a, T, V>;
            /// Returned from [`ScopedArena::iter_mut`]
            pub type IterMut<'a, T, V = crate::version::DefaultVersion> = imp::IterMut<'a, T, V>;
            /// Returned from [`ScopedArena::into_iter`]
            pub type IntoIter<T, V = crate::version::DefaultVersion> = imp::IntoIter<T, V>;

            /// Returned from [`ScopedArena::drain`]
            pub type Drain<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Drain<'a, T, V>;
            /// Returned from [`ScopedArena::drain_filter`]
            pub type DrainFilter<'a, 'scope, T, F, V = crate::version::DefaultVersion> = imp::DrainFilter<'a, T, V, F>;

            /// Returned from [`ScopedArena::keys`]
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
        /// see [`base::sparse`](crate::base::sparse) for details
        pub mod hop {
            use core::ops::*;
            use crate::base::hop as imp;

            /// Returned from [`ScopedArena::iter`]
            pub type Iter<'a, T, V = crate::version::DefaultVersion> = imp::Iter<'a, T, V>;
            /// Returned from [`ScopedArena::iter_mut`]
            pub type IterMut<'a, T, V = crate::version::DefaultVersion> = imp::IterMut<'a, T, V>;
            /// Returned from [`ScopedArena::into_iter`]
            pub type IntoIter<T, V = crate::version::DefaultVersion> = imp::IntoIter<T, V>;

            /// Returned from [`ScopedArena::drain`]
            pub type Drain<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Drain<'a, T, V>;
            /// Returned from [`ScopedArena::drain_filter`]
            pub type DrainFilter<'a, 'scope, T, F, V = crate::version::DefaultVersion> = imp::DrainFilter<'a, T, V, F>;

            /// Returned from [`ScopedArena::keys`]
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
        /// see [`base::dense`](crate::base::sparse) for details
        pub mod dense {
            use core::ops::*;
            use crate::base::dense as imp;

            /// Returned from [`ScopedArena::iter`]
            pub type Iter<'a, T> = core::slice::Iter<'a, T>;
            /// Returned from [`ScopedArena::iter_mut`]
            pub type IterMut<'a, T> = core::slice::IterMut<'a, T>;
            /// Returned from [`ScopedArena::into_iter`]
            pub type IntoIter<T> = std::vec::IntoIter<T>;

            /// Returned from [`ScopedArena::drain`]
            pub type Drain<'a, 'scope, T, V = crate::version::DefaultVersion> = imp::Drain<'a, T, pui_core::scoped::Scoped<'scope>, V>;
            /// Returned from [`ScopedArena::drain_filter`]
            pub type DrainFilter<'a, 'scope, T, F, V = crate::version::DefaultVersion> = imp::DrainFilter<'a, T, pui_core::scoped::Scoped<'scope>, V, F>;

            /// Returned from [`ScopedArena::keys`]
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

/// A key into scoped arenas
///
/// This type only exists to make error messages easier to parse
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct ScopedKey<'scope, V>(pub crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V>);

impl<'scope, V> From<crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V>> for ScopedKey<'scope, V> {
    fn from(key: crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V>) -> Self { Self(key) }
}

impl<'scope, V> From<ScopedKey<'scope, V>> for crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> {
    fn from(ScopedKey(key): ScopedKey<'scope, V>) -> Self { key }
}

impl<'scope, V> Borrow<crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V>> for ScopedKey<'scope, V> {
    fn borrow(&self) -> &crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> { &self.0 }
}

impl<'scope, V> Borrow<ScopedKey<'scope, V>> for crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> {
    fn borrow(&self) -> &ScopedKey<'scope, V> { unsafe { core::mem::transmute(self) } }
}

impl<'scope, V> BorrowMut<crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V>> for ScopedKey<'scope, V> {
    fn borrow_mut(&mut self) -> &mut crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> { &mut self.0 }
}

impl<'scope, V> BorrowMut<ScopedKey<'scope, V>> for crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> {
    fn borrow_mut(&mut self) -> &mut ScopedKey<'scope, V> { unsafe { core::mem::transmute(self) } }
}

impl<'scope, V> AsRef<crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V>> for ScopedKey<'scope, V> {
    fn as_ref(&self) -> &crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> { &self.0 }
}

impl<'scope, V> AsRef<ScopedKey<'scope, V>> for crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> {
    fn as_ref(&self) -> &ScopedKey<'scope, V> { unsafe { core::mem::transmute(self) } }
}

impl<'scope, V> AsMut<crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V>> for ScopedKey<'scope, V> {
    fn as_mut(&mut self) -> &mut crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> { &mut self.0 }
}

impl<'scope, V> AsMut<ScopedKey<'scope, V>> for crate::Key<pui_vec::Id<pui_core::scoped::ScopedToken<'scope>>, V> {
    fn as_mut(&mut self) -> &mut ScopedKey<'scope, V> { unsafe { core::mem::transmute(self) } }
}

impl<'scope, V: Version> ArenaKey<pui_core::scoped::Scoped<'scope>, V> for ScopedKey<'scope, V::Save> {
    fn validate_ident<'a>(
        &self,
        ident: &'a pui_core::scoped::Scoped<'scope>,
        validator: Validator<'a>,
    ) -> CompleteValidator<'a> {
        ArenaKey::<pui_core::scoped::Scoped<'scope>, V>::validate_ident(&self.0, ident, validator)
    }
    fn index(&self) -> usize { ArenaKey::<pui_core::scoped::Scoped<'scope>, V>::index(&self.0) }
    fn version(&self) -> Option<V::Save> { ArenaKey::<pui_core::scoped::Scoped<'scope>, V>::version(&self.0) }
}

impl<'scope, V: Version> BuildArenaKey<pui_core::scoped::Scoped<'scope>, V> for ScopedKey<'scope, V::Save> {
    unsafe fn new_unchecked(index: usize, save: V::Save, ident: &pui_core::scoped::Scoped<'scope>) -> Self {
        Self(BuildArenaKey::<pui_core::scoped::Scoped<'scope>, V>::new_unchecked(
            index, save, ident,
        ))
    }
}
