#![no_std]
#![forbid(missing_docs, clippy::missing_safety_doc)]

//! A set of very efficient, and very customizable arenas that
//! can elide bounds checks wherever possible.
//!

#[doc(hidden)]
pub extern crate alloc as std;

pub mod version;

pub mod base {
    pub mod dense;
    pub mod hop;
    pub mod sparse;
}

#[cfg(feature = "scoped")]
pub mod scoped;
#[cfg(feature = "slab")]
pub mod slab;
#[cfg(feature = "slot_map")]
pub mod slot_map;

#[doc(hidden)]
#[cfg(feature = "pui")]
pub use {core, pui_core, pui_vec};

#[derive(Clone, Copy)]
pub struct TrustedIndex(usize);

impl TrustedIndex {
    #[inline]
    pub unsafe fn new(index: usize) -> Self { Self(index) }
}

pub struct SetOnDrop<'a>(&'a mut bool);

impl Drop for SetOnDrop<'_> {
    fn drop(&mut self) { *self.0 = true; }
}

impl SetOnDrop<'_> {
    fn defuse(self) { core::mem::forget(self) }
}

#[macro_export]
#[cfg(feature = "pui")]
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
        $item_vis type Identifier = $crate::pui_core::dynamic::Dynamic<super::$name>;
        $item_vis type Key = key::Key<$crate::pui_vec::Id<$crate::pui_core::dynamic::DynamicToken<super::$name>>, Version>;

        $item_vis type BaseArena<T> = imp::Arena<T, Identifier, Version>;
        $item_vis type BaseVacantEntry<'a, T> = imp::VacantEntry<'a, T, Identifier, Version>;

        $item_vis struct Arena<T>($item_vis imp::Arena<T, Identifier, Version>);
        $item_vis struct VacantEntry<'a, T>($item_vis imp::VacantEntry<'a, T, Identifier, Version>);

        $item_vis type Entries<'a, T> = imp::Entries<'a, T, Identifier, Version, Key>;
        $item_vis type EntriesMut<'a, T> = imp::EntriesMut<'a, T, Identifier, Version, Key>;
        $item_vis type IntoEntries<T> = imp::IntoEntries<T, Identifier, Version, Key>;

        impl<T> VacantEntry<'_, T> {
            pub fn key(&self) -> Key { self.0.key() }

            pub fn insert(self, value: T) -> Key { self.0.insert(value) }
        }

        impl<T> $crate::core::default::Default for Arena<T> {
            fn default() -> Self { Self::new() }
        }

        impl<T> Arena<T> {
            pub fn new() -> Self {
                Self(BaseArena::with_ident(super::$name::oneshot()))
            }

            pub fn ident(&self) -> &Identifier { self.0.ident() }
            pub fn $slots(&self) -> usize { self.0.$slots() }
            pub fn capacity(&self) -> usize { self.0.capacity() }
            pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
            pub fn parse_key(&self, index: usize) -> Option<Key> { self.0.parse_key(index) }
            pub fn vacant_entry(&mut self) -> VacantEntry<'_, T> { VacantEntry(self.0.vacant_entry()) }
            pub fn insert(&mut self, value: T) -> Key { self.0.insert(value) }
            pub fn contains(&self, key: Key) -> bool { self.0.contains(key) }
            pub fn remove(&mut self, key: Key) -> T { self.0.remove(key) }
            pub fn try_remove(&mut self, key: Key) -> Option<T> { self.0.try_remove(key) }
            pub fn delete(&mut self, key: Key) -> bool { self.0.delete(key) }
            pub fn get(&self, key: Key) -> Option<&T> { self.0.get(key) }
            pub fn get_mut(&mut self, key: Key) -> Option<&mut T> { self.0.get_mut(key) }
            pub unsafe fn get_unchecked(&self, index: usize) -> &T { self.0.get_unchecked(index) }
            pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T { self.0.get_unchecked_mut(index) }
            pub fn delete_all(&mut self) { self.0.delete_all() }
            pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, f: F) { self.0.retain(f) }
            pub fn keys(&self) -> Keys<'_ $(, $keys)?> { self.0.keys() }
            pub fn iter(&self) -> Iter<'_, T> { self.0.iter() }
            pub fn iter_mut(&mut self) -> IterMut<'_, T> { self.0.iter_mut() }
            pub fn drain(&mut self) -> Drain<'_, T> { self.0.drain() }
            pub fn drain_filter<F: FnMut(&mut T) -> bool>(&mut self, filter: F) -> DrainFilter<'_, T, F> { self.0.drain_filter(filter) }
            pub fn entries(&self) -> Entries<'_, T> { self.0.entries() }
            pub fn entries_mut(&mut self) -> EntriesMut<'_, T> { self.0.entries_mut() }
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
        $mod_vis mod sparse {
            use $crate::core::ops::*;
            use $crate::base::sparse as imp;
            use $crate::base::sparse as key;

            $item_vis type Version = $version;
            $item_vis type Iter<'a, T> = imp::Iter<'a, T, Version>;
            $item_vis type IterMut<'a, T> = imp::IterMut<'a, T, Version>;
            $item_vis type IntoIter<T> = imp::IntoIter<T, Version>;

            $item_vis type Drain<'a, T> = imp::Drain<'a, T, Version>;
            $item_vis type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Version, F>;

            $item_vis type Keys<'a, T> = imp::Keys<'a, T, Identifier, Version, Key>;

            $crate::__newtype! {
                @forward
                ($item_vis) $name
                slots: slots
                T
            }
        }

        $mod_vis mod hop {
            use $crate::core::ops::*;
            use $crate::base::hop as imp;
            use $crate::base::hop as key;

            $item_vis type Version = $version;
            $item_vis type Iter<'a, T> = imp::Iter<'a, T, Version>;
            $item_vis type IterMut<'a, T> = imp::IterMut<'a, T, Version>;
            $item_vis type IntoIter<T> = imp::IntoIter<T, Version>;

            $item_vis type Drain<'a, T> = imp::Drain<'a, T, Identifier, Version>;
            $item_vis type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Identifier, Version, F>;

            $item_vis type Keys<'a, T> = imp::Keys<'a, T, Identifier, Version, Key>;

            $crate::__newtype! {
                @forward
                ($item_vis) $name
                slots: len
                T
            }
        }

        $mod_vis mod dense {
            use $crate::core::ops::*;
            use $crate::base::dense as imp;
            use $crate::base::sparse as key;

            $item_vis type Version = $version;
            $item_vis type Iter<'a, T> = $crate::core::slice::Iter<'a, T>;
            $item_vis type IterMut<'a, T> = $crate::core::slice::IterMut<'a, T>;
            $item_vis type IntoIter<T> = $crate::std::vec::IntoIter<T>;

            $item_vis type Drain<'a, T> = imp::Drain<'a, T, Identifier, Version>;
            $item_vis type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Identifier, Version, F>;

            $item_vis type Keys<'a> = imp::Keys<'a, Identifier, Version, Key>;

            $crate::__newtype! {
                @forward
                ($item_vis) $name
                slots: len
            }
        }
    };
}
