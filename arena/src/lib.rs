#![no_std]
#![forbid(missing_docs)]
#![deny(clippy::missing_safety_doc)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::transmute_ptr_to_ptr)]
// FIXME - the docs in this crate a *very* minimal, and need to be expanded upon

//! A set of very efficient, and very customizable arenas that
//! can elide bounds checks wherever possible.
//!
//! This crate is heavily inspired by crates like [`slotmap`](https://crates.io/crate/slotmap)
//! and [`slab`](https://crates.io/crate/slab).
//!
//! `pui-arena` provides a set of collections that allow you to insert and delete
//! items in at least amortized `O(1)`, access elements in `O(1)`. It also provides
//! the tools required to avoid the [`ABA` problem](https://en.wikipedia.org/wiki/ABA_problem).
//!
//! You can think of the collections in `pui-arena` as a `HashMap`/`BTreeMap` where
//! the arena manages the keys, and provides a very efficient way to access elements.
//!
//! # Why use `pui-arena` over alternatives
//!
//! `pui-arena` allows you to minimize overhead wherever possible, and fully customize
//! the arenas. This allows you to use an api like `slab` or `slotmap` based on how
//! you use the api. (There are also newtypes featured-gated by the features `slab`
//! and `slotmap` that implement a similar interface to those two crates).
//!
//! If you use the `pui`/`scoped` feature, then you can also eliminate bounds checks
//! entirely, which can be a huge performance save in performance sensitive regions.
//!
//! `pui-arena` also provides a more features than competitors, such as a vacant entry
//! api for versioned arenas, and `drain_filter` for all arenas.
//!
//! # Choosing [`sparse`](base::sparse), [`hop`](base::hop), or [`dense`](base::dense)
//!
//! * If you want fast insertion/deletion/acccess and don't care about iteration speed,
//! use [`sparse`](base::sparse).
//!
//! * If you want fast iteration speed above all else, use [`dense`](base::dense)
//!
//! * If you want reasonable iteration speed and also fast access/delete, or if [`dense`](base::dense)
//! is to memory heavy, use [`hop`](base::hop)
//!
//! You can read about the details of how each works in the corrosponding module docs
//!
//! # Performance characteristics
//!
//! ## Speed
//!
//! all of the collections in `pui-arena` allow you to
//! * insert elements in amortized `O(1)`
//! * delete/access elements in `O(1)`
//! * guarantee that keys *never* get invalidated unless `remove` is called
//!
//! ## Memory
//!
//! For each `Arena<T, _, V>` where `V: Version`, the memory overhead is as follows:
//! * [`sparse`](base::sparse) [`Arena`](base::sparse::Arena) -
//!     `size_of(V) + max(size_of(T), size_of(usize))` per slot
//! * [`hop`](base::hop) [`Arena`](base::hop::Arena) -
//!     `size_of(V) + max(size_of(T), 3 * size_of(usize))` per slot
//! * [`dense`](base::dense) [`Arena`](base::dense::Arena) -
//!     `size_of(V) + size_of(usize)` per slot,
//!     and `size_of(usize) + size_of(T)` per value
//!
//! ## Implementation Details
//!
//! The core of this crate is the the [`Version`](version::Version) trait,
//! the [`ArenaKey`](ArenaKey) trait, and the [`BuildArenaKey`](BuildArenaKey) trait.
//!
//! `Version` specifies the behavior of the arenas.
//! `pui-arena` provides three implementations,
//! see [`Version`](version::Version) for more details:
//!
//! * [`DefaultVersion`](version::DefaultVersion)
//!     * Ensures that all keys produced by `insert` are unique
//!     * backed by a `u32`, so it may waste space for small values
//!         * technically if items are inserted/removed many times,
//!             slots will be "leaked", and iteraton performance may degrade
//!             but, this is unlikely, unless the same slot is reused over
//!             2 billion times
//! * [`TinyVersion`](version::TinyVersion) -
//!     * Ensures that all keys produced by `insert` are unique
//!     * backed by a `u8`, if items are inserted/removed many times,
//!         slots will be "leaked", and iteraton performance may degrade
//! * [`Unversioned`](version::Unversioned) -
//!     * Keys produced by `insert` are not guartneed to be unique
//!     * slots will never be "leaked"
//!
//! [`ArenaKey`] specifies the behavior of keys into arenas.
//! `pui-arena` provides a number of implementations. See [`ArenaKey`]
//! for details.
//!
//! * [`usize`] - allows accessing a given slot directly, with no regard for it's version
//!     * Note: when I say "with no regard for it's version", it still checks the version
//!         to see if the slot is occupied, but it has no means of checking if a slot
//!         a value was re-inserted into the same slot
//! * [`Key<K, _>`](Key) - allows accessing a slot specified by `K`, and checks the generation
//!     of the slot before providing a value.
//!     * `K` can be one of the other keys listed here (except for `ScopedKey`)
//! * [`TrustedIndex`] - allows accessing a given slot directly, with no regard for it's version
//!     * elides bounds checks, but is unsafe to construct
//!     * This one should be used with care, if at all. It is better to use the `pui` feature
//!         and use `pui_vec::Id` instead. It is safe, and also guartnees bound check elision
//! * [`ScopedKey<'_, _>`](scoped::ScopedKey) - only allows access into scoped arenas
//!     (otherwise identical to `Key`)
//!
//! enabled with the `pui` feature
//!
//! * [`pui_vec::Id`] - allows accessing a given slot directly, with no regard for it's version
//!     * elides bounds checks
//!
//! [`BuildArenaKey`] specifies how arenas should create keys, all implementors of [`ArenaKey`]
//! provided by this crate also implement [`BuildArenaKey`] except for [`TrustedIndex`].
//!
//! # Custom arenas
//!
//! You can newtype arenas with the [`newtype`] macro, or the features: `slab`, `slotmap`, or `scoped`.
//!
//! * [`slab`] - provides a similar api to the [`slab` crate](https://crates.io/crate/slab)
//!     * uses `usize` keys, and [`Unversioned`](version::Unversioned) slots
//! * [`slotmap`] - provides a similar api to the [`slab` crate](https://crates.io/crate/slotmap)
//!     * uses [`Key<usize>`](Key) keys, and [`DefaultVersion`](version::DefaultVersion) slots
//! * [`scoped`] - provides newtyped arenas that use `pui_core::scoped` to elide bounds checks
//!     * uses [`scoped::ScopedKey<'_, _>`](scoped::ScopedKey) keys,
//!     and is generic over the version
//! * [`newtype`] - creates a set of newtyped arenas with the module structure of `base`
//!     * These arenas elide bounds checks, in favor of id checks, which are cheaper,
//!     and depending on your backing id, can be no check at all!
//!     (see [`pui_core::scalar_allocator`] details)
//!
//! ```
//! // Because the backing id type is `()`, there are no bounds checks when using
//! // this arena!
//! # use inner::*; mod inner {
//! pui_arena::newtype! {
//!     pub struct MyCustomArena;
//! }
//! # }
//!
//! let my_sparse_arena = sparse::Arena::<()>::new();
//! // You can also use `dense` or `hop`
//! // let my_dense_arena = dense::Arena::<()>::new();
//! // let my_hop_arena = hop::Arena::<()>::new();
//! ```
//!
//! Becomes something like
//!
//! ```ignore
//! pui_core::scalar_allocator! {
//!     struct MyCustomArena;
//! }
//!
//! mod sparse {
//!     pub(super) Arena(pub(super) pui_arena::base::sparse::Arena<...>);
//!
//!     /// more type aliases here
//! }
//!
//! mod dense {
//!     pub(super) Arena(pub(super) pui_arena::base::dense::Arena<...>);
//!
//!     /// more type aliases here
//! }
//!
//! mod hop {
//!     pub(super) Arena(pub(super) pui_arena::base::hop::Arena<...>);
//!
//!     /// more type aliases here
//! }
//!
//! let my_sparse_arena = sparse::Arena::<()>::new();
//! // You can also use `dense` or `hop`
//! // let my_dense_arena = dense::Arena::<()>::new();
//! // let my_hop_arena = hop::Arena::<()>::new();
//! ```
//!
//! Where each `Arena` newtype has a simplified api, and better error messages.

#[doc(hidden)]
pub extern crate alloc as std;

pub mod version;

mod arena_access;
pub use arena_access::{ArenaKey, BuildArenaKey, CompleteValidator, Key, Validator};

/// the core implementations of different types of arenas
pub mod base {
    pub mod dense;
    pub mod hop;
    pub mod sparse;
}

#[cfg(feature = "scoped")]
#[cfg_attr(docsrs, doc(cfg(feature = "scoped")))]
pub mod scoped;
/// a reimplementation of [`slab`](https://docs.rs/slab/) in terms
/// of the generic arenas in [`base`]
#[cfg(feature = "slab")]
#[cfg_attr(docsrs, doc(cfg(feature = "slab")))]
pub mod slab;
#[cfg(feature = "slotmap")]
#[cfg_attr(docsrs, doc(cfg(feature = "slotmap")))]
pub mod slotmap;

#[doc(hidden)]
#[cfg(feature = "pui")]
pub use {core, pui_core, pui_vec};

/// An index that's guaranteed to be in bounds of the arena it's used on
#[derive(Clone, Copy)]
pub struct TrustedIndex(usize);

impl TrustedIndex {
    /// Create a new `TrustedIndex`
    ///
    /// # Safety
    ///
    /// This `index` must be in bounds on all arenas this `Self` is used on
    #[inline]
    pub unsafe fn new(index: usize) -> Self { Self(index) }
}

struct SetOnDrop<'a>(&'a mut bool);

impl Drop for SetOnDrop<'_> {
    fn drop(&mut self) { *self.0 = true; }
}

impl SetOnDrop<'_> {
    fn defuse(self) { core::mem::forget(self) }
}

/// Create newtype of all the arenas in [`base`]
///
/// The module structure here is identical to [`crate::base`], and
/// you can look there for detailed documentation about the types.
/// Each implementation of `SlotMap` will have all the methods from the
/// corrosponding `Arena`, and those that take or produce generic keys
/// will instead take/produce a `Key`.
///
/// In each module, you'll find an `Arena` newtype (with one public field),
/// a `VacantEntry` newtype (again with one public field). These are thin
/// wrappers around their generic counterparts. Their only serve the purpose
/// of making error messages easier to parse, and use a default `Key`.
/// You will also find a vareity of type aliases for various iterators, and
/// for the default `Key` type for ease of use.
///
/// If you want to access the raw backing `Arena`/`VacantEntry`, you still can,
/// it is the only public field of each scoped arena/vacant entry.
#[macro_export]
#[cfg(feature = "pui")]
#[cfg_attr(docsrs, doc(cfg(feature = "pui")))]
macro_rules! newtype {
    (
        $(#[$meta:meta])*
        $( pub $(( $($vis:tt)* ))?  )? struct $name:ident;
        $(type Version = $version:ty;)?
    ) => {
        $crate::pui_core::scalar_allocator! {
            $(#[$meta])*
            $( pub $(( $($vis)* ))?  )? struct $name;
        }

        $crate::__newtype! { @resolve_vis $( pub $(( $($vis)* ))?  )? $name, $($version,)? $crate::version::DefaultVersion }
    };
    (
        $(#[$meta:meta])*
        $( pub $(( $($vis:tt)* ))?  )? struct $name:ident($inner:ty);
        $(type Version = $version:ty;)?
    ) => {
        $crate::pui_core::scalar_allocator! {
            $(#[$meta])*
            $( pub $(( $($vis)* ))?  )? struct $name($inner);
        }

        $crate::__newtype! { @resolve_vis $( pub $(( $($vis)* ))?  )? $name, $($version,)? $crate::version::DefaultVersion }
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "pui")]
macro_rules! __newtype {
    (@resolve_vis $name:ident, $default_version:ty $(, $extra:ty)?) => {
        $crate::__newtype! {  @build_module (pub(self)) (pub(super)) $name, $default_version }
    };
    (@resolve_vis pub $name:ident, $default_version:ty $(, $extra:ty)?) => {
        $crate::__newtype! {  @build_module (pub) (pub) $name, $default_version }
    };
    (@resolve_vis pub(self) $name:ident, $default_version:ty $(, $extra:ty)?) => {
        $crate::__newtype! {  @build_module (pub(self)) (pub(super)) $name, $default_version }
    };
    (@resolve_vis pub(crate) $name:ident, $default_version:ty $(, $extra:ty)?) => {
        $crate::__newtype! {  @build_module (pub(crate)) (pub(crate)) $name, $default_version }
    };
    (@resolve_vis pub(in $($path:tt)*) $name:ident, $default_version:ty $(, $extra:ty)?) => {
        $crate::__newtype! {  @build_module (pub(in $($path)*)) (pub(in super::$($path)*)) $name, $default_version }
    };
    (
        @forward
        ($item_vis:vis) $name:ident
        slots: $slots:ident
        $($keys:ident)?
    ) => {
        /// The backing identifier for [`Arena`]
        $item_vis type Identifier = $crate::pui_core::dynamic::Dynamic<super::$name>;
        /// The key for [`Arena`]
        $item_vis type Key = $crate::Key<$crate::pui_vec::Id<$crate::pui_core::dynamic::DynamicToken<super::$name>>, <Version as $crate::version::Version>::Save>;

        /// The backing arena for [`Arena`]
        $item_vis type BaseArena<T> = imp::Arena<T, Identifier, Version>;
        /// The backing vacant entry for [`VacantEntry`]
        $item_vis type BaseVacantEntry<'a, T> = imp::VacantEntry<'a, T, Identifier, Version>;

        /// A newtyped arena
        $item_vis struct Arena<T>($item_vis imp::Arena<T, Identifier, Version>);
        /// A newtyped vacant entry
        $item_vis struct VacantEntry<'a, T>($item_vis imp::VacantEntry<'a, T, Identifier, Version>);

        /// Returned from [`Arena::entries`]
        $item_vis type Entries<'a, T> = imp::Entries<'a, T, Identifier, Version, Key>;
        /// Returned from [`Arena::entries_mut`]
        $item_vis type EntriesMut<'a, T> = imp::EntriesMut<'a, T, Identifier, Version, Key>;
        /// Returned from [`Arena::into_entries`]
        $item_vis type IntoEntries<T> = imp::IntoEntries<T, Identifier, Version, Key>;

        impl<T> VacantEntry<'_, T> {
            /// see [`VacantEntry::key`](imp::VacantEntry::key)
            pub fn key(&self) -> Key { self.0.key() }

            /// see [`VacantEntry::insert`](imp::VacantEntry::insert)
            pub fn insert(self, value: T) -> Key { self.0.insert(value) }
        }

        impl<T> $crate::core::default::Default for Arena<T> {
            fn default() -> Self { Self::new() }
        }

        impl<T> Arena<T> {
            /// Create a new slab
            pub fn new() -> Self {
                Self(BaseArena::with_ident(super::$name::oneshot()))
            }
            /// see [`Arena::is_empty`](imp::Arena::is_empty)
            pub fn is_empty(&self) -> bool { self.0.is_empty() }
            /// see [`Arena::len`](imp::Arena::is_empty)
            pub fn len(&self) -> usize { self.0.len() }
            /// see [`Arena::capacity`](imp::Arena::capacity)
            pub fn capacity(&self) -> usize { self.0.capacity() }
            /// see [`Arena::reserve`](imp::Arena::reserve)
            pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
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
            pub fn keys(&self) -> Keys<'_ $(, $keys)?> { self.0.keys() }
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

        impl<T> $crate::core::iter::IntoIterator for Arena<T> {
            type IntoIter = IntoIter<T>;
            type Item = T;

            fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
        }

        impl<T> Index<Key> for Arena<T> {
            type Output = T;

            fn index(&self, key: Key) -> &Self::Output { &self.0[key] }
        }

        impl<T> IndexMut<Key> for Arena<T> {
            fn index_mut(&mut self, key: Key) -> &mut Self::Output { &mut self.0[key] }
        }
    };
    (@build_module ($mod_vis:vis) ($item_vis:vis) $name:ident, $version:ty) => {
        /// a sparse arena
        ///
        /// see [`pui_arena::base::sparse`](sparse::imp) for details
        $mod_vis mod sparse {
            use $crate::core::ops::*;
            #[doc(hidden)]
            pub(super) use $crate::base::sparse as imp;

            /// The version for [`Arena`]
            $item_vis type Version = $version;
            /// Returned from [`Arena::iter`]
            $item_vis type Iter<'a, T> = imp::Iter<'a, T, Version>;
            /// Returned from [`Arena::iter_mut`]
            $item_vis type IterMut<'a, T> = imp::IterMut<'a, T, Version>;
            /// Returned from [`Arena::into_iter`]
            $item_vis type IntoIter<T> = imp::IntoIter<T, Version>;

            /// Returned from [`Arena::drain`]
            $item_vis type Drain<'a, T> = imp::Drain<'a, T, Version>;
            /// Returned from [`Arena::drain_filter`]
            $item_vis type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Version, F>;

            /// Returned from [`Arena::keys`]
            $item_vis type Keys<'a, T> = imp::Keys<'a, T, Identifier, Version, Key>;

            $crate::__newtype! {
                @forward
                ($item_vis) $name
                slots: slots
                T
            }
        }

        /// a hop arena
        ///
        /// see [`pui_arena::base::hop`](hop::imp) for details
        $mod_vis mod hop {
            use $crate::core::ops::*;
            #[doc(hidden)]
            pub(super) use $crate::base::hop as imp;

            /// The version for [`Arena`]
            $item_vis type Version = $version;
            /// Returned from [`Arena::iter`]
            $item_vis type Iter<'a, T> = imp::Iter<'a, T, Version>;
            /// Returned from [`Arena::iter_mut`]
            $item_vis type IterMut<'a, T> = imp::IterMut<'a, T, Version>;
            /// Returned from [`Arena::into_iter`]
            $item_vis type IntoIter<T> = imp::IntoIter<T, Version>;

            /// Returned from [`Arena::drain`]
            $item_vis type Drain<'a, T> = imp::Drain<'a, T, Version>;
            /// Returned from [`Arena::drain_filter`]
            $item_vis type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Version, F>;

            /// Returned from [`Arena::keys`]
            $item_vis type Keys<'a, T> = imp::Keys<'a, T, Identifier, Version, Key>;

            $crate::__newtype! {
                @forward
                ($item_vis) $name
                slots: len
                T
            }
        }

        /// a dense arena
        ///
        /// see [`pui_arena::base::dense`](dense::imp) for details
        $mod_vis mod dense {
            use $crate::core::ops::*;
            #[doc(hidden)]
            pub(super) use $crate::base::dense as imp;

            /// The version for [`Arena`]
            $item_vis type Version = $version;
            /// Returned from [`Arena::iter`]
            $item_vis type Iter<'a, T> = $crate::core::slice::Iter<'a, T>;
            /// Returned from [`Arena::iter_mut`]
            $item_vis type IterMut<'a, T> = $crate::core::slice::IterMut<'a, T>;
            /// Returned from [`Arena::into_iter`]
            $item_vis type IntoIter<T> = $crate::std::vec::IntoIter<T>;

            /// Returned from [`Arena::drain`]
            $item_vis type Drain<'a, T> = imp::Drain<'a, T, Identifier, Version>;
            /// Returned from [`Arena::drain_filter`]
            $item_vis type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Identifier, Version, F>;

            /// Returned from [`Arena::keys`]
            $item_vis type Keys<'a> = imp::Keys<'a, Identifier, Version, Key>;

            $crate::__newtype! {
                @forward
                ($item_vis) $name
                slots: len
            }
        }
    };
}
