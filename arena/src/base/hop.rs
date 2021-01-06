use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

use crate::version::Version;

mod imp;
pub use imp::*;

mod iter_unchecked;
use iter_unchecked::IteratorUnchecked;

#[derive(Clone, Copy)]
pub struct Key<Id, V = crate::version::DefaultVersion> {
    id: Id,
    version: V,
}

#[derive(Debug, Clone)]
pub struct Arena<T, I = (), V: Version = crate::version::DefaultVersion> {
    slots: PuiVec<Slot<T, V>, I>,
    num_elements: usize,
}

pub trait ArenaAccess<I, V: Version> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool;

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T>;

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T>;

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T>;
}

impl<K: ?Sized + ArenaAccess<I, V>, I, V: Version> ArenaAccess<I, V> for &K {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool { K::contained_in(self, arena) }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { K::get(self, arena) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> { K::get_mut(self, arena) }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> { K::try_remove(self, arena) }
}

pub trait BuildArenaKey<I, V: Version> {
    unsafe fn new_unchecked(index: usize, version: V::Save, ident: &I) -> Self;
}

impl<I, V: Version> ArenaAccess<I, V> for usize {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        arena.slots.get(*self).map_or(false, Slot::is_occupied)
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[self].get() }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> { arena.slots[self].get_mut() }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        let index = *self;
        let slot = &mut arena.slots[index];

        if slot.is_occupied() {
            Some(unsafe { arena.remove_unchecked(index) })
        } else {
            None
        }
    }
}

impl<I, V: Version> BuildArenaKey<I, V> for usize {
    unsafe fn new_unchecked(index: usize, _: V::Save, _: &I) -> Self { index }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> ArenaAccess<I, V> for pui_vec::Id<I::Token> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        arena.ident().owns_token(self.token()) && arena.slots[self].is_occupied()
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[self].get() }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> { arena.slots[self].get_mut() }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        let index = self.get();
        let slot = &mut arena.slots[self];

        if slot.is_occupied() {
            unsafe { Some(arena.remove_unchecked(index)) }
        } else {
            None
        }
    }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> BuildArenaKey<I, V> for pui_vec::Id<I::Token> {
    unsafe fn new_unchecked(index: usize, _: V::Save, ident: &I) -> Self {
        pui_vec::Id::new_unchecked(index, ident.token())
    }
}

impl<I, V: Version> ArenaAccess<I, V> for Key<usize, V::Save> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        let saved = self.version;
        arena
            .slots
            .get(self.id)
            .map_or(false, |slot| slot.version().equals_saved(saved))
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[self.id].get_with(self.version) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> {
        arena.slots[self.id].get_mut_with(self.version)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        let index = self.id;
        let slot = &mut arena.slots[self.id];

        if slot.version().equals_saved(self.version) {
            unsafe { Some(arena.remove_unchecked(index)) }
        } else {
            None
        }
    }
}

impl<I, V: Version> BuildArenaKey<I, V> for Key<usize, V::Save> {
    unsafe fn new_unchecked(index: usize, version: V::Save, _: &I) -> Self { Key { id: index, version } }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> ArenaAccess<I, V> for Key<pui_vec::Id<I::Token>, V::Save> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        let saved = self.version;
        arena.ident().owns_token(self.id.token())
            && arena
                .slots
                .get(self.id.get())
                .map_or(false, |slot| slot.version().equals_saved(saved))
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[&self.id].get_with(self.version) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> {
        arena.slots[&self.id].get_mut_with(self.version)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        let index = self.id.get();
        let slot = &mut arena.slots[&self.id];

        if slot.version().equals_saved(self.version) {
            unsafe { Some(arena.remove_unchecked(index)) }
        } else {
            None
        }
    }
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

impl<T> Arena<T> {
    pub fn new() -> Self { Self::with_ident(()) }
}

impl<T, V: Version> Arena<T, (), V> {
    pub fn clear(&mut self) {
        self.slots.vec_mut().clear();
        let _: usize = self.slots.push(Slot::SENTINEL);
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    pub fn with_ident(ident: I) -> Self {
        Self {
            num_elements: 0,
            slots: PuiVec::from_raw_parts(std::vec![Slot::SENTINEL], ident),
        }
    }

    pub fn ident(&self) -> &I { self.slots.ident() }

    pub fn len(&self) -> usize { self.num_elements }

    pub fn capacity(&self) -> usize { self.slots.capacity() }

    pub fn reserve(&mut self, additional: usize) { self.slots.reserve(additional) }

    #[inline]
    pub fn parse_key<K: BuildArenaKey<I, V>>(&self, index: usize) -> Option<K> {
        let slot = self.slots.get(index)?;
        slot.parse_key(index, self.slots.ident())
    }

    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I, V> { self.__vacant_entry() }

    pub fn insert<K: BuildArenaKey<I, V>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    pub fn remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<T> { key.try_remove(self) }

    pub fn contains<K: ArenaAccess<I, V>>(&self, key: K) -> bool { key.contained_in(self) }

    pub fn get<K: ArenaAccess<I, V>>(&self, key: K) -> Option<&T> { key.get(self) }

    pub fn get_mut<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<&mut T> { key.get_mut(self) }

    pub fn remove_all(&mut self) { self.retain(|_| false) }

    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        let mut i = 0;

        for _ in 0..self.num_elements {
            unsafe {
                let slot = self.slots.get_unchecked_mut(i);
                if slot.is_vacant() {
                    i = 1 + slot.other_end();
                }

                let value = self.slots.get_unchecked_mut(i).get_mut_unchecked();

                if !f(value) {
                    self.remove_unchecked(i);
                }
            }

            i += 1;
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
                len: self.num_elements,
                slots: iter_unchecked::Iter::new(&self.slots).enumerate(),
            },
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T, V> {
        IterMut {
            slots: OccupiedMut {
                len: self.num_elements,
                slots: iter_unchecked::IterMut::new(&mut self.slots).enumerate(),
            },
        }
    }

    pub fn entries<K: BuildArenaKey<I, V>>(&self) -> Entries<'_, T, I, V, K> {
        Entries {
            slots: Occupied {
                len: self.num_elements,
                slots: iter_unchecked::Iter::new(&self.slots).enumerate(),
            },
            ident: self.slots.ident(),
            key: PhantomData,
        }
    }

    pub fn entries_mut<K: BuildArenaKey<I, V>>(&mut self) -> EntriesMut<'_, T, I, V, K> {
        let (ident, slots) = self.slots.as_mut_parts();
        EntriesMut {
            slots: OccupiedMut {
                len: self.num_elements,
                slots: iter_unchecked::IterMut::new(slots).enumerate(),
            },
            ident,
            key: PhantomData,
        }
    }

    pub fn into_entries<K: BuildArenaKey<I, V>>(self) -> IntoEntries<T, I, V, K> {
        let (ident, slots) = self.slots.into_raw_parts();
        IntoEntries {
            slots: IntoOccupied {
                len: self.num_elements,
                slots: iter_unchecked::IntoIter::new(slots).enumerate(),
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
            slots: IntoOccupied {
                len: self.num_elements,
                slots: iter_unchecked::IntoIter::new(self.slots.into_raw_parts().1).enumerate(),
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

struct OccupiedBase<I> {
    len: usize,
    slots: iter_unchecked::Enumerate<I>,
}

type Occupied<'a, T, V> = OccupiedBase<iter_unchecked::Iter<'a, Slot<T, V>>>;
type OccupiedMut<'a, T, V> = OccupiedBase<iter_unchecked::IterMut<'a, Slot<T, V>>>;
type IntoOccupied<T, V> = OccupiedBase<iter_unchecked::IntoIter<Slot<T, V>>>;

impl<I: IteratorUnchecked> Iterator for OccupiedBase<I> {
    type Item = (usize, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        unsafe {
            let index = self.slots.index();
            let slot = self.slots.peek();
            if slot.is_vacant() {
                let skip = slot.other_end().wrapping_sub(index).wrapping_add(1);
                self.slots.advance(skip);
            }
            Some(self.slots.next())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (self.len, Some(self.len)) }

    fn count(self) -> usize { self.len }
}

impl<I: IteratorUnchecked> DoubleEndedIterator for OccupiedBase<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        unsafe {
            let index = self.slots.index_back();
            let slot = self.slots.peek_back();
            if slot.is_vacant() {
                let skip = index.wrapping_sub(slot.other_end());
                self.slots.advance_back(skip);
            }
            Some(self.slots.next_back())
        }
    }
}

pub struct Keys<'a, T, I, V: Version, K> {
    entries: Entries<'a, T, I, V, K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Keys<'a, T, I, V, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> { self.entries.next().map(|(key, _)| key) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.entries.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.entries.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Keys<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.entries.next_back().map(|(key, _)| key) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for Keys<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Keys<'_, T, I, V, K> {}

#[inline(always)]
fn value<T, U, V>((_, (_, v)): (T, (U, V))) -> V { v }
#[inline(always)]
unsafe fn entry<I, V: Version, T, K: BuildArenaKey<I, V>>(ident: &I) -> impl '_ + FnOnce((usize, (V, T))) -> (K, T) {
    #[inline(always)]
    move |(index, (version, value))| (K::new_unchecked(index, version.save(), ident), value)
}

pub struct Iter<'a, T, V: Version> {
    slots: Occupied<'a, T, V>,
}

impl<'a, T, V: Version> Iterator for Iter<'a, T, V> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(value) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, V: Version> DoubleEndedIterator for Iter<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(value) }
}
impl<T, V: Version> ExactSizeIterator for Iter<'_, T, V> {}
impl<T, V: Version> core::iter::FusedIterator for Iter<'_, T, V> {}

pub struct IterMut<'a, T, V: Version> {
    slots: OccupiedMut<'a, T, V>,
}

impl<'a, T, V: Version> Iterator for IterMut<'a, T, V> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(value) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, V: Version> DoubleEndedIterator for IterMut<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(value) }
}
impl<T, V: Version> ExactSizeIterator for IterMut<'_, T, V> {}
impl<T, V: Version> core::iter::FusedIterator for IterMut<'_, T, V> {}

pub struct IntoIter<T, V: Version> {
    slots: IntoOccupied<T, V>,
}

impl<T, V: Version> Iterator for IntoIter<T, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(value) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, V: Version> DoubleEndedIterator for IntoIter<T, V> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(value) }
}
impl<T, V: Version> ExactSizeIterator for IntoIter<T, V> {}
impl<T, V: Version> core::iter::FusedIterator for IntoIter<T, V> {}

pub struct Entries<'a, T, I, V: Version, K> {
    slots: Occupied<'a, T, V>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Entries<'a, T, I, V, K> {
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(unsafe { entry(self.ident) }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Entries<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(unsafe { entry(self.ident) }) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for Entries<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Entries<'_, T, I, V, K> {}

pub struct EntriesMut<'a, T, I, V: Version, K> {
    slots: OccupiedMut<'a, T, V>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for EntriesMut<'a, T, I, V, K> {
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(unsafe { entry(self.ident) }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for EntriesMut<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(unsafe { entry(self.ident) }) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for EntriesMut<'_, T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for EntriesMut<'_, T, I, V, K> {}

pub struct IntoEntries<T, I, V: Version, K> {
    slots: IntoOccupied<T, V>,
    ident: I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for IntoEntries<T, I, V, K> {
    type Item = (K, T);

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(unsafe { entry(&self.ident) }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }

    fn last(mut self) -> Option<Self::Item> { self.next_back() }

    fn count(self) -> usize { self.slots.count() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for IntoEntries<T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.slots.next_back().map(unsafe { entry(&self.ident) }) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> ExactSizeIterator for IntoEntries<T, I, V, K> {}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for IntoEntries<T, I, V, K> {}

#[cfg(test)]
mod test {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn basic() {
        let mut arena = Arena::new();

        let a: usize = arena.insert(0);
        assert_eq!(a, 1);
        assert_eq!(arena[a], 0);
        assert_eq!(arena.remove(a), 0);
        assert_eq!(arena.get(a), None);

        let b: usize = arena.insert(10);
        assert_eq!(b, 1);
        assert_eq!(arena[b], 10);

        let b: usize = arena.insert(20);
        assert_eq!(b, 2);
        assert_eq!(arena[b], 20);
        assert_eq!(arena.remove(a), 10);
        assert_eq!(arena.get(a), None);

        let c: usize = arena.insert(30);
        assert_eq!(c, 1);
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
