//! A reimplementation of [`slab`](https://docs.rs/slab/) in terms
//! of the generic arenas in [`base`](crate::base)
//!
//! The module structure here is identical to [`crate::base`], and
//! you can look there for detailed documentation about the types.
//! Each implementation of `Slab` will have all the methods from the
//! corrosponding `Arena`, and those that take or produce generic keys
//! will instead take/produce `usize` (which is type alised to `Key`).
//!
//! In each module, you'll find an `Slab` newtype (with one public field),
//! a `VacantEntry` newtype (again with one public field). These are thin
//! wrappers around their generic counterparts. Their only serve the purpose
//! of making error messages easier to parse, and use a default `Key`.
//! You will also find a vareity of type aliases for various iterators, and
//! for the default `Key` type for ease of use.
//!
//! If you want to access the raw backing `Arena`/`VacantEntry`, you still can,
//! it is the only public field of each slotmap/vacant entry.

macro_rules! imp_slab {
    (
        new $($const:ident)?: $new:expr,
        slots: $slots:ident,
        ($($value:ty)?)
    ) => {
        /// a slab
        #[derive(Debug, Clone)]
        #[repr(transparent)]
        pub struct Slab<T>(pub Arena<T, (), Unversioned>);

        /// a vacant entry into [`Slab`]
        pub struct VacantEntry<'a, T>(pub imp::VacantEntry<'a, T, (), Unversioned>);

        /// The key for [`Slab`]
        pub type Key = usize;

        /// Returned from [`Slab::entries`]
        pub type Entries<'a, T> = imp::Entries<'a, T, (), Unversioned, usize>;
        /// Returned from [`Slab::entries_mut`]
        pub type EntriesMut<'a, T> = imp::EntriesMut<'a, T, (), Unversioned, usize>;
        /// Returned from [`Slab::into_entries`]
        pub type IntoEntries<T> = imp::IntoEntries<T, (), Unversioned, usize>;

        impl<T> VacantEntry<'_, T> {
            /// see [`VacantEntry::key`](imp::VacantEntry::key)
            pub fn key(&self) -> usize { self.0.key() }

            /// see [`VacantEntry::insert`](imp::VacantEntry::insert)
            pub fn insert(self, value: T) -> usize { self.0.insert(value) }
        }

        impl<T> Default for Slab<T> {
            fn default() -> Self { Self::new() }
        }

        impl<T> Slab<T> {
            /// Create a new slab
            pub $($const)? fn new() -> Self { Self($new) }
            /// see [`Arena::is_empty`](imp::Arena::is_empty)
            pub fn is_empty(&self) -> bool { self.0.is_empty() }
            /// see [`Arena::len`](imp::Arena::is_empty)
            pub fn len(&self) -> usize { self.0.len() }
            /// see [`Arena::capacity`](imp::Arena::capacity)
            pub fn capacity(&self) -> usize { self.0.capacity() }
            /// see [`Arena::reserve`](imp::Arena::reserve)
            pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
            /// see [`Arena::clear`](imp::Arena::reserve)
            pub fn clear(&mut self) { self.0.clear(); }
            /// see [`Arena::vacant_entry`](imp::Arena::vacant_entry)
            pub fn vacant_entry(&mut self) -> VacantEntry<'_, T> { VacantEntry(self.0.vacant_entry()) }
            /// see [`Arena::insert`](imp::Arena::insert)
            pub fn insert(&mut self, value: T) -> Key { self.0.insert(value) }
            /// see [`Arena::contains`](imp::Arena::contains)
            pub fn contains(&self, key: Key) -> bool { self.0.contains(key) }
            /// see [`Arena::remove`](imp::Arena::remove)
            pub fn remove(&mut self, key: Key) -> T { self.0.remove(key) }
            /// see [`Arena::try_remove`](imp::Arena::try_remove)
            pub fn try_remove(&mut self, key: Key) -> Option<T> { self.0.try_remove(key) }
            /// see [`Arena::delete`](imp::Arena::delete)
            pub fn delete(&mut self, key: Key) -> bool { self.0.delete(key) }
            /// see [`Arena::get`](imp::Arena::get)
            pub fn get(&self, key: Key) -> Option<&T> { self.0.get(key) }
            /// see [`Arena::get_mut`](imp::Arena::get_mut)
            pub fn get_mut(&mut self, key: Key) -> Option<&mut T> { self.0.get_mut(key) }
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
            pub fn keys(&self) -> Keys<'_ $(, $value)?> { self.0.keys() }
            /// see [`Arena::iter`](imp::Arena::iter)
            pub fn iter(&self) -> Iter<'_, T> { self.0.iter() }
            /// see [`Arena::iter_mut`](imp::Arena::iter_mut)
            pub fn iter_mut(&mut self) -> IterMut<'_, T> { self.0.iter_mut() }
            /// see [`Arena::drain`](imp::Arena::drain)
            pub fn drain(&mut self) -> Drain<'_, T> { self.0.drain() }
            /// see [`Arena::drain_filter`](imp::Arena::drain_filter)
            pub fn drain_filter<F: FnMut(&mut T) -> bool>(&mut self, filter: F) -> DrainFilter<'_, T, F> { self.0.drain_filter(filter) }
            /// see [`Arena::entries`](imp::Arena::entries)
            pub fn entries(&self) -> Entries<'_, T> { self.0.entries() }
            /// see [`Arena::entries_mut`](imp::Arena::entries_mut)
            pub fn entries_mut(&mut self) -> EntriesMut<'_, T> { self.0.entries_mut() }
            /// see [`Arena::into_entries`](imp::Arena::into_entries)
            pub fn into_entries(self) -> IntoEntries<T> { self.0.into_entries() }
        }

        impl<T> IntoIterator for Slab<T> {
            type IntoIter = IntoIter<T>;
            type Item = T;

            fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
        }

        impl<T> Index<Key> for Slab<T> {
            type Output = T;

            fn index(&self, key: Key) -> &Self::Output { &self.0[key] }
        }

        impl<T> IndexMut<Key> for Slab<T> {
            fn index_mut(&mut self, key: Key) -> &mut Self::Output { &mut self.0[key] }
        }
    };
}

/// a dense slab
///
/// see [`base::dense`](crate::base::dense) for details
pub mod dense {
    use core::ops::{Index, IndexMut};

    use crate::{
        base::dense::{self as imp, Arena},
        version::Unversioned,
    };

    /// Returned from [`Slab::iter`]
    pub type Iter<'a, T> = core::slice::Iter<'a, T>;
    /// Returned from [`Slab::iter_mut`]
    pub type IterMut<'a, T> = core::slice::IterMut<'a, T>;
    /// Returned from [`Slab::into_iter`]
    pub type IntoIter<T> = std::vec::IntoIter<T>;

    /// Returned from [`Slab::drain`]
    pub type Drain<'a, T> = imp::Drain<'a, T, (), Unversioned>;
    /// Returned from [`Slab::drain_filter`]
    pub type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, (), Unversioned, F>;

    /// Returned from [`Slab::keys`]
    pub type Keys<'a> = imp::Keys<'a, (), Unversioned, Key>;

    imp_slab! {
        new: Arena::with_ident(()),
        slots: len,
        ()
    }
}

/// a hop slab
///
/// see [`base::hop`](crate::base::hop) for details
pub mod hop {
    use core::ops::{Index, IndexMut};

    use crate::{
        base::hop::{self as imp, Arena},
        version::Unversioned,
    };

    /// Returned from [`Slab::iter`]
    pub type Iter<'a, T> = imp::Iter<'a, T, Unversioned>;
    /// Returned from [`Slab::iter_mut`]
    pub type IterMut<'a, T> = imp::IterMut<'a, T, Unversioned>;
    /// Returned from [`Slab::into_iter`]
    pub type IntoIter<T> = imp::IntoIter<T, Unversioned>;

    /// Returned from [`Slab::drain`]
    pub type Drain<'a, T> = imp::Drain<'a, T, Unversioned>;
    /// Returned from [`Slab::drain_filter`]
    pub type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Unversioned, F>;

    /// Returned from [`Slab::keys`]
    pub type Keys<'a, T> = imp::Keys<'a, T, (), Unversioned, Key>;

    imp_slab! {
        new: Arena::with_ident(()),
        slots: len,
        (T)
    }
}

/// a sparse slab
///
/// see [`base::sparse`](crate::base::sparse) for details
pub mod sparse {
    use core::ops::{Index, IndexMut};

    use crate::{
        base::sparse::{self as imp, Arena},
        version::Unversioned,
    };

    /// Returned from [`Slab::iter`]
    pub type Iter<'a, T> = imp::Iter<'a, T, Unversioned>;
    /// Returned from [`Slab::iter_mut`]
    pub type IterMut<'a, T> = imp::IterMut<'a, T, Unversioned>;
    /// Returned from [`Slab::into_iter`]
    pub type IntoIter<T> = imp::IntoIter<T, Unversioned>;

    /// Returned from [`Slab::drain`]
    pub type Drain<'a, T> = imp::Drain<'a, T, Unversioned>;
    /// Returned from [`Slab::drain_filter`]
    pub type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Unversioned, F>;

    /// Returned from [`Slab::keys`]
    pub type Keys<'a, T> = imp::Keys<'a, T, (), Unversioned, Key>;

    imp_slab! {
        new const: Arena::INIT,
        slots: slots,
        (T)
    }
}
