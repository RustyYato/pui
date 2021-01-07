use core::{
    marker::PhantomData,
    mem::{replace, ManuallyDrop},
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

use crate::version::{DefaultVersion, Version};

pub trait ArenaAccess<I, V: Version> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool;

    fn index(&self) -> usize;
}

pub trait BuildArenaKey<I, V: Version>: ArenaAccess<I, V> {
    unsafe fn new_unchecked(index: usize, save: V::Save, ident: &I) -> Self;
}

impl<K: ?Sized + ArenaAccess<I, V>, I, V: Version> ArenaAccess<I, V> for &K {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool { K::contained_in(self, arena) }

    fn index(&self) -> usize { K::index(self) }
}

impl<I, V: Version> ArenaAccess<I, V> for usize {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        arena.slots.get(*self).map_or(false, |slot| slot.version.is_full())
    }

    fn index(&self) -> usize { *self }
}

impl<I, V: Version> BuildArenaKey<I, V> for usize {
    unsafe fn new_unchecked(index: usize, _: V::Save, _: &I) -> Self { index }
}

impl<I, V: Version> ArenaAccess<I, V> for crate::TrustedIndex {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        unsafe { arena.slots.get_unchecked(self.0).version.is_full() }
    }

    fn index(&self) -> usize { self.0 }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> ArenaAccess<I, V> for pui_vec::Id<I::Token> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        arena.ident().owns_token(self.token()) && arena.slots[self].version.is_full()
    }

    fn index(&self) -> usize { self.get() }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> BuildArenaKey<I, V> for pui_vec::Id<I::Token> {
    unsafe fn new_unchecked(index: usize, _: V::Save, ident: &I) -> Self {
        pui_vec::Id::new_unchecked(index, ident.token())
    }
}

impl<I, V: Version> ArenaAccess<I, V> for Key<usize, V::Save> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        let version = self.version;
        arena
            .slots
            .get(self.id)
            .map_or(false, |slot| slot.version.equals_saved(version))
    }

    fn index(&self) -> usize { self.id }
}

impl<I, V: Version> BuildArenaKey<I, V> for Key<usize, V::Save> {
    unsafe fn new_unchecked(index: usize, version: V::Save, _: &I) -> Self { Key { id: index, version } }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> ArenaAccess<I, V> for Key<pui_vec::Id<I::Token>, V::Save> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        let version = self.version;
        arena.ident().owns_token(self.id.token())
            && arena
                .slots
                .get(self.id.get())
                .map_or(false, |slot| slot.version.equals_saved(version))
    }

    fn index(&self) -> usize { self.id.get() }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> BuildArenaKey<I, V> for Key<pui_vec::Id<I::Token>, V::Save> {
    unsafe fn new_unchecked(index: usize, version: V::Save, ident: &I) -> Self {
        Key {
            id: pui_vec::Id::new_unchecked(index, ident.token()),
            version,
        }
    }
}

impl<I, V: Version> ArenaAccess<I, V> for Key<crate::TrustedIndex, V::Save> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        let version = self.version;
        unsafe { arena.slots.get_unchecked(self.id.0).version.equals_saved(version) }
    }

    fn index(&self) -> usize { self.id.0 }
}

union Data<T> {
    value: ManuallyDrop<T>,
    next: usize,
}

struct Slot<T, V: Version> {
    version: V,
    data: Data<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Key<Id, V = crate::version::DefaultVersion> {
    id: Id,
    version: V,
}

#[derive(Debug, Clone)]
pub struct Arena<T, I = (), V: Version = DefaultVersion> {
    slots: PuiVec<Slot<T, V>, I>,
    next: usize,
}

pub struct VacantEntry<'a, T, I, V: Version = DefaultVersion> {
    arena: &'a mut Arena<T, I, V>,
    new_next: usize,
}

impl<T, V: Version> Drop for Slot<T, V> {
    fn drop(&mut self) {
        if self.version.is_full() {
            unsafe { ManuallyDrop::drop(&mut self.data.value) }
        }
    }
}

impl<T, V: Version> Slot<T, V> {
    unsafe fn remove_unchecked(&mut self, index: usize, next: &mut usize) -> T {
        let value = ManuallyDrop::take(&mut self.data.value);
        if let Some(next_version) = self.version.mark_empty() {
            self.version = next_version;

            self.data = Data {
                next: replace(next, index),
            };
        }
        value
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Arena<T> {
    pub const fn new() -> Self { Self::INIT }
}

impl<T, V: Version> Arena<T, (), V> {
    pub const INIT: Self = Self {
        slots: PuiVec::new(()),
        next: 0,
    };

    pub fn clear(&mut self) {
        self.next = 0;
        self.slots.vec_mut().clear();
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    pub fn with_ident(ident: I) -> Self {
        Self {
            slots: PuiVec::new(ident),
            next: 0,
        }
    }

    pub fn ident(&self) -> &I { self.slots.ident() }

    pub fn slots(&self) -> usize { self.slots.len() }

    pub fn capacity(&self) -> usize { self.slots.capacity() }

    pub fn reserve(&mut self, additional: usize) { self.slots.reserve(additional) }
}

impl<Id, V> Key<Id, V> {
    pub const fn id(&self) -> &Id { &self.id }

    pub const fn version(&self) -> &V { &self.version }
}

impl<T, I, V: Version> VacantEntry<'_, T, I, V> {
    pub fn key<K: BuildArenaKey<I, V>>(&self) -> K {
        unsafe {
            K::new_unchecked(
                self.arena.next,
                self.arena
                    .slots
                    .get_unchecked(self.arena.next)
                    .version
                    .mark_full()
                    .save(),
                self.arena.ident(),
            )
        }
    }

    pub fn insert<K: BuildArenaKey<I, V>>(self, value: T) -> K {
        let slot = unsafe { self.arena.slots.get_unchecked_mut(self.arena.next) };
        slot.data = Data {
            value: ManuallyDrop::new(value),
        };
        slot.version = unsafe { slot.version.mark_full() };
        let version = unsafe { slot.version.save() };
        let index = self.arena.next;
        self.arena.next = self.new_next;

        unsafe { K::new_unchecked(index, version, self.arena.ident()) }
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    #[inline]
    pub fn parse_key<K: BuildArenaKey<I, V>>(&self, index: usize) -> Option<K> {
        let slot = self.slots.get(index)?;
        if slot.version.is_full() {
            Some(unsafe { K::new_unchecked(index, slot.version.save(), self.slots.ident()) })
        } else {
            None
        }
    }

    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I, V> {
        #[cold]
        #[inline(never)]
        pub fn allocate_vacant_slot<T, I, V: Version>(this: &mut Arena<T, I, V>) {
            this.next = this.slots.len();
            let _: usize = this.slots.push(Slot {
                version: V::EMPTY,
                data: Data {
                    next: this.next.wrapping_add(1),
                },
            });
        }

        if self.slots.len() == self.next {
            allocate_vacant_slot(self);
        }

        let slot = unsafe { self.slots.get_unchecked_mut(self.next) };

        VacantEntry {
            new_next: unsafe { slot.data.next },
            arena: self,
        }
    }

    pub fn insert<K: BuildArenaKey<I, V>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    pub fn remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn contains<K: ArenaAccess<I, V>>(&self, key: K) -> bool { key.contained_in(self) }

    pub fn try_remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<T> {
        if self.contains(&key) {
            let index = key.index();
            Some(unsafe {
                self.slots
                    .get_unchecked_mut(index)
                    .remove_unchecked(index, &mut self.next)
            })
        } else {
            None
        }
    }

    pub fn get<K: ArenaAccess<I, V>>(&self, key: K) -> Option<&T> {
        if self.contains(&key) {
            unsafe {
                let index = key.index();
                let slot = self.slots.get_unchecked(index);
                Some(&*slot.data.value)
            }
        } else {
            None
        }
    }

    pub fn get_mut<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<&mut T> {
        if self.contains(&key) {
            unsafe {
                let index = key.index();
                let slot = self.slots.get_unchecked_mut(index);
                Some(&mut *slot.data.value)
            }
        } else {
            None
        }
    }

    pub fn remove_all(&mut self) { self.retain(|_| false) }

    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        for i in 0..self.slots.len() {
            match self.get_mut(unsafe { crate::TrustedIndex::new(i) }) {
                Some(value) => unsafe {
                    if !f(value) {
                        self.slots.get_unchecked_mut(i).remove_unchecked(i, &mut self.next);
                    }
                },
                _ => (),
            }
        }
    }

    pub fn keys<K: BuildArenaKey<I, V>>(&self) -> Keys<'_, T, I, V, K> {
        Keys {
            entries: self.entries(),
        }
    }

    pub fn iter(&self) -> Iter<'_, T, V> {
        Iter {
            slots: Occupied {
                slots: self.slots.iter(),
            },
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T, V> {
        IterMut {
            slots: Occupied {
                slots: self.slots.iter_mut(),
            },
        }
    }

    pub fn entries<K: BuildArenaKey<I, V>>(&self) -> Entries<'_, T, I, V, K> {
        let ident = self.ident();

        Entries {
            slots: Occupied {
                slots: self.slots.iter().enumerate(),
            },
            ident,
            key: PhantomData,
        }
    }

    pub fn entries_mut<K: BuildArenaKey<I, V>>(&mut self) -> EntriesMut<'_, T, I, V, K> {
        let (ident, slots) = self.slots.as_mut_parts();

        EntriesMut {
            slots: Occupied {
                slots: slots.iter_mut().enumerate(),
            },
            ident,
            key: PhantomData,
        }
    }

    pub fn into_entries<K: BuildArenaKey<I, V>>(self) -> IntoEntries<T, I, V, K> {
        let (ident, slots) = self.slots.into_raw_parts();

        IntoEntries {
            slots: Occupied {
                slots: slots.into_iter().enumerate(),
            },
            ident,
            key: PhantomData,
        }
    }
}

impl<T, I, V: Version> IntoIterator for Arena<T, I, V> {
    type Item = T;
    type IntoIter = IntoIter<T, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            slots: Occupied {
                slots: self.slots.into_raw_parts().1.into_iter(),
            },
        }
    }
}

impl<T, I, V: Version, K: ArenaAccess<I, V>> Index<K> for Arena<T, I, V> {
    type Output = T;

    fn index(&self, key: K) -> &Self::Output { self.get(key).expect("Tried to access `Arena` with a stale `Key`") }
}

impl<T, I, V: Version, K: ArenaAccess<I, V>> IndexMut<K> for Arena<T, I, V> {
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

impl<T, I, V: Version> Extend<T> for Arena<T, I, V> {
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        iter.for_each(move |value| drop::<usize>(self.vacant_entry().insert(value)));
    }
}

use core::fmt;

impl<T: Clone, V: Version> Clone for Slot<T, V> {
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            data: if self.version.is_full() {
                Data {
                    value: unsafe { self.data.value.clone() },
                }
            } else {
                Data {
                    next: unsafe { self.data.next },
                }
            },
        }
    }

    fn clone_from(&mut self, source: &Self) {
        if self.version.is_full() && source.version.is_full() {
            self.version = source.version;
            unsafe {
                self.data.value.clone_from(&source.data.value);
            }
        } else {
            *self = source.clone()
        }
    }
}

impl<T: fmt::Debug, V: Version + fmt::Debug> fmt::Debug for Slot<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.version.is_full() {
            f.debug_struct("Occupied")
                .field("version", &self.version)
                .field("value", unsafe { &*self.data.value })
                .finish()
        } else {
            f.debug_struct("Vacant")
                .field("version", &self.version)
                .field("next", unsafe { &self.data.next })
                .finish()
        }
    }
}

struct Occupied<I> {
    slots: I,
}

trait AsSlot {
    type Item;
    type Version: Version;

    fn as_slot(&self) -> &Slot<Self::Item, Self::Version>;
}

impl<T, V: Version> AsSlot for Slot<T, V> {
    type Item = T;
    type Version = V;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self }
}

impl<T, V: Version> AsSlot for &mut Slot<T, V> {
    type Item = T;
    type Version = V;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self }
}

impl<T, V: Version> AsSlot for &Slot<T, V> {
    type Item = T;
    type Version = V;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self }
}

impl<T: AsSlot> AsSlot for (usize, T) {
    type Item = T::Item;
    type Version = T::Version;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item, Self::Version> { self.1.as_slot() }
}

impl<I: Iterator> Iterator for Occupied<I>
where
    I::Item: AsSlot,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> { self.slots.by_ref().find(|slot| slot.as_slot().version.is_full()) }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Occupied<I>
where
    I::Item: AsSlot,
{
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.by_ref().rfind(|slot| slot.as_slot().version.is_full()) }
}

pub struct Keys<'a, T, I, V: Version, K> {
    entries: Entries<'a, T, I, V, K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Keys<'a, T, I, V, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> { self.entries.next().map(|(key, _)| key) }
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Keys<'a, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.entries.next_back().map(|(key, _)| key) }
}

pub struct Iter<'a, T, V: Version> {
    slots: Occupied<core::slice::Iter<'a, Slot<T, V>>>,
}

impl<'a, T, V: Version> Iterator for Iter<'a, T, V> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|slot| unsafe { &*slot.data.value }) }
}

impl<T, V: Version> DoubleEndedIterator for Iter<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(|slot| unsafe { &*slot.data.value }) }
}

pub struct IterMut<'a, T, V: Version> {
    slots: Occupied<core::slice::IterMut<'a, Slot<T, V>>>,
}

impl<'a, T, V: Version> Iterator for IterMut<'a, T, V> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|slot| unsafe { &mut *slot.data.value }) }
}

impl<T, V: Version> DoubleEndedIterator for IterMut<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|slot| unsafe { &mut *slot.data.value })
    }
}

pub struct IntoIter<T, V: Version> {
    slots: Occupied<std::vec::IntoIter<Slot<T, V>>>,
}

impl<T, V: Version> Iterator for IntoIter<T, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.next().map(|slot| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }
}

impl<T, V: Version> DoubleEndedIterator for IntoIter<T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|slot| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }
}

pub struct Entries<'a, T, I, V: Version, K> {
    slots: Occupied<core::iter::Enumerate<core::slice::Iter<'a, Slot<T, V>>>>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Entries<'a, T, I, V, K> {
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next()
            .map(|(index, slot)| unsafe { (K::new_unchecked(index, slot.version.save(), ident), &*slot.data.value) })
    }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Entries<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next_back()
            .map(|(index, slot)| unsafe { (K::new_unchecked(index, slot.version.save(), ident), &*slot.data.value) })
    }
}

pub struct EntriesMut<'a, T, I, V: Version, K> {
    slots: Occupied<core::iter::Enumerate<core::slice::IterMut<'a, Slot<T, V>>>>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for EntriesMut<'a, T, I, V, K> {
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next().map(|(index, slot)| unsafe {
            (
                K::new_unchecked(index, slot.version.save(), ident),
                &mut *slot.data.value,
            )
        })
    }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for EntriesMut<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next_back().map(|(index, slot)| unsafe {
            (
                K::new_unchecked(index, slot.version.save(), ident),
                &mut *slot.data.value,
            )
        })
    }
}

pub struct IntoEntries<T, I, V: Version, K> {
    slots: Occupied<core::iter::Enumerate<std::vec::IntoIter<Slot<T, V>>>>,
    ident: I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for IntoEntries<T, I, V, K> {
    type Item = (K, T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next().map(|(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut slot.data.value);
            (K::new_unchecked(index, slot.version.save(), ident), value)
        })
    }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for IntoEntries<T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next_back().map(|(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut slot.data.value);
            (K::new_unchecked(index, slot.version.save(), ident), value)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn basic() {
        let mut arena = Arena::new();

        let a: usize = arena.insert(0);
        assert_eq!(a, 0);
        assert_eq!(arena[a], 0);
        assert_eq!(arena.remove(a), 0);
        assert_eq!(arena.get(a), None);

        let b: usize = arena.insert(10);
        assert_eq!(b, 0);
        assert_eq!(arena[b], 10);

        let b: usize = arena.insert(20);
        assert_eq!(b, 1);
        assert_eq!(arena[b], 20);
        assert_eq!(arena.remove(a), 10);
        assert_eq!(arena.get(a), None);

        let c: usize = arena.insert(30);
        assert_eq!(c, 0);
        assert_eq!(arena[c], 30);
        assert_eq!(arena.remove(b), 20);
        assert_eq!(arena.get(b), None);
        assert_eq!(arena.remove(c), 30);
        assert_eq!(arena.get(c), None);
    }

    #[test]
    fn basic_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        for i in ins_values.len()..10 {
            ins_values.push(arena.insert(i * 100));
        }
    }

    #[test]
    fn basic_retain() {
        let mut arena = Arena::new();
        for i in 0..10 {
            let _: usize = arena.insert(i);
        }
        arena.retain(|&mut i| i % 3 == 0);
        let mut items = arena.iter().copied().collect::<Vec<_>>();
        items.sort_unstable();
        assert_eq!(items, [0, 3, 6, 9]);
    }

    #[test]
    fn iter_keys_insert_only() {
        let mut arena = Arena::new();
        let ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_keys = arena.keys().collect::<Vec<usize>>();
        iter_keys.sort_unstable();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_rev_insert_only() {
        let mut arena = Arena::new();
        let ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_keys = arena.keys().rev().collect::<Vec<usize>>();
        iter_keys.sort_unstable();

        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_with_removal() {
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        let mut iter_keys = arena.keys().collect::<Vec<usize>>();
        iter_keys.sort_unstable();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        ins_keys.sort_unstable();
        let mut iter_keys = arena.keys().rev().collect::<Vec<usize>>();
        iter_keys.sort_unstable();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_with_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        for i in ins_keys.len()..10 {
            ins_keys.push(arena.insert(i * 100));
        }
        let mut iter_keys = arena.keys().collect::<Vec<usize>>();
        let mut rev_iter_keys = arena.keys().rev().collect::<Vec<usize>>();

        // the order that the keys come out doesn't matter,
        // so put them in a canonical order
        ins_keys.sort_unstable();
        iter_keys.sort_unstable();
        rev_iter_keys.sort_unstable();

        assert_eq!(ins_keys, iter_keys);
        assert_eq!(ins_keys, rev_iter_keys);
    }

    #[test]
    fn iter_values_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values = arena.iter().copied().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_rev_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values = arena.iter().copied().rev().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values = arena.iter().copied().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values = arena.iter().copied().rev().collect::<Vec<_>>();
        iter_values.sort_unstable();
        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_with_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        for i in ins_values.len()..10 {
            ins_values.push(arena.insert(i * 100));
        }
        let mut iter_values = arena.iter().copied().collect::<Vec<usize>>();
        let mut rev_iter_values = arena.iter().copied().rev().collect::<Vec<usize>>();

        // the order that the iter come out doesn't matter,
        // so put them in a canonical order
        iter_values.sort_unstable();
        rev_iter_values.sort_unstable();

        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
        assert_eq!(rev_iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
    }

    #[test]
    fn iter_values_mut_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_mut_rev_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_mut_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_mut_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<_>>();
        iter_values_mut.sort_unstable();
        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_mut_with_reinsertion() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        for i in ins_values.len()..10 {
            ins_values.push(arena.insert(i * 100));
        }
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<usize>>();
        let mut rev_iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<usize>>();

        // the order that the iter come out doesn't matter,
        // so put them in a canonical order
        iter_values_mut.sort_unstable();
        rev_iter_values_mut.sort_unstable();

        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
        assert_eq!(rev_iter_values_mut, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
    }

    #[test]
    fn into_iter_values_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut into_iter_values = arena.into_iter().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn into_iter_values_rev_insert_only() {
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut into_iter_values = arena.into_iter().rev().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn into_iter_values_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut into_iter_values = arena.into_iter().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn into_iter_values_rev_with_removal() {
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut into_iter_values = arena.into_iter().rev().collect::<Vec<_>>();
        into_iter_values.sort_unstable();
        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn into_iter_values_with_reinsertion() {
        let mk_arena = || {
            let mut arena = Arena::new();
            let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
            for i in (0..ins_values.len()).rev().step_by(3) {
                let key = ins_values.remove(i);
                arena.remove(key);
            }
            for i in ins_values.len()..10 {
                ins_values.push(arena.insert(i * 100));
            }
            arena
        };
        let mut into_iter_values = mk_arena().into_iter().collect::<Vec<usize>>();
        let mut rev_into_iter_values = mk_arena().into_iter().rev().collect::<Vec<usize>>();

        // the order that the iter come out doesn't matter,
        // so put them in a canonical order
        into_iter_values.sort_unstable();
        rev_into_iter_values.sort_unstable();

        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
        assert_eq!(rev_into_iter_values, [10, 20, 40, 50, 70, 80, 600, 700, 800, 900]);
    }
}
