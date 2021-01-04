use core::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

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

pub trait BuildArenaKey<I, V: Version>: ArenaAccess<I, V> {
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
            .map_or(false, |slot| slot.version.equals_saved(saved))
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[self.id].get_with(self.version) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> {
        arena.slots[self.id].get_mut_with(self.version)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        let index = self.id;
        let slot = &mut arena.slots[self.id];

        if slot.version.equals_saved(self.version) {
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
                .map_or(false, |slot| slot.version.equals_saved(saved))
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[&self.id].get_with(self.version) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> {
        arena.slots[&self.id].get_mut_with(self.version)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        let index = self.id.get();
        let slot = &mut arena.slots[&self.id];

        if slot.version.equals_saved(self.version) {
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

#[derive(Clone, Copy)]
struct FreeNode {
    next: usize,
    prev: usize,
    end: usize,
}

union Data<T> {
    value: ManuallyDrop<T>,
    free: FreeNode,
}

struct Slot<T, V: Version> {
    version: V,
    data: Data<T>,
}

#[derive(Clone, Copy)]
pub struct Key<Id, V = crate::version::DefaultVersion> {
    id: Id,
    version: V,
}

#[derive(Debug, Clone)]
pub struct Arena<T, I = (), V: Version = DefaultVersion> {
    slots: PuiVec<Slot<T, V>, I>,
    num_elements: usize,
}

pub struct VacantEntry<'a, T, I, V: Version = DefaultVersion> {
    arena: &'a mut Arena<T, I, V>,
    index: usize,
    updated_gen: V,
    free: FreeNode,
}

impl<T, V: Version> Drop for Slot<T, V> {
    fn drop(&mut self) {
        if self.is_occupied() {
            unsafe { ManuallyDrop::drop(&mut self.data.value) }
        }
    }
}

impl<T, V: Version> Slot<T, V> {
    const SENTINEL: Self = Slot {
        version: V::EMPTY,
        data: Data {
            free: FreeNode {
                next: 0,
                prev: 0,
                end: 0,
            },
        },
    };

    fn is_occupied(&self) -> bool { self.version.is_full() }

    fn is_vacant(&self) -> bool { !self.is_occupied() }

    fn get(&self) -> Option<&T> {
        if self.version.is_full() {
            Some(unsafe { &*self.data.value })
        } else {
            None
        }
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        if self.version.is_full() {
            Some(unsafe { &mut *self.data.value })
        } else {
            None
        }
    }

    fn get_with(&self, saved: V::Save) -> Option<&T> {
        if self.version.equals_saved(saved) {
            Some(unsafe { &*self.data.value })
        } else {
            None
        }
    }

    fn get_mut_with(&mut self, saved: V::Save) -> Option<&mut T> {
        if self.version.equals_saved(saved) {
            Some(unsafe { &mut *self.data.value })
        } else {
            None
        }
    }
}

impl<'a, T, I, V: Version> VacantEntry<'a, T, I, V> {
    pub fn key<K: BuildArenaKey<I, V>>(&self) -> K {
        unsafe { K::new_unchecked(self.index, self.updated_gen.save(), self.arena.slots.ident()) }
    }

    pub fn insert<K: BuildArenaKey<I, V>>(self, value: T) -> K {
        unsafe {
            let slot = self.arena.slots.get_unchecked_mut(self.index);
            slot.data = Data {
                value: ManuallyDrop::new(value),
            };
            slot.version = self.updated_gen;
            self.arena.num_elements += 1;

            self.arena.remove_slot_from_freelist(self.index, self.free);

            K::new_unchecked(self.index, self.updated_gen.save(), self.arena.slots.ident())
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

    unsafe fn freelist(&mut self, idx: usize) -> &mut FreeNode { &mut self.slots.get_unchecked_mut(idx).data.free }

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
        unsafe fn allocate_new_node<T, I, V: Version>(arena: &mut Arena<T, I, V>, index: usize) {
            arena.slots.push::<usize>(Slot {
                version: V::EMPTY,
                data: Data {
                    free: FreeNode {
                        next: 0,
                        prev: 0,
                        end: index,
                    },
                },
            });

            arena.freelist(0).next = index;
        }

        unsafe {
            let head = self.freelist(0).next;

            if head == 0 {
                // if there are no elements in the freelist

                let index = self.slots.len();
                allocate_new_node(self, index);

                VacantEntry {
                    arena: self,
                    index,
                    updated_gen: V::EMPTY,
                    free: FreeNode {
                        next: 0,
                        prev: 0,
                        end: index,
                    },
                }
            } else {
                let slot = self.slots.get_unchecked_mut(head);
                let updated_gen = slot.version.mark_full();
                let free = slot.data.free;

                VacantEntry {
                    arena: self,
                    index: head,
                    updated_gen,
                    free,
                }
            }
        }
    }

    pub fn insert<K: BuildArenaKey<I, V>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    pub fn remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<T> {
        let value = key.try_remove(self)?;
        self.num_elements -= 1;
        Some(value)
    }

    pub unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        let slot = self.slots.get_unchecked_mut(index);
        let value = ManuallyDrop::take(&mut slot.data.value);
        self.insert_slot_into_freelist(index);
        value
    }

    #[inline(always)]
    unsafe fn remove_slot_from_freelist(&mut self, index: usize, free: FreeNode) {
        if free.end != index {
            // if there are more items in the block, pop this node from the freelist

            let index = index.wrapping_add(1);
            *self.freelist(index) = free;

            self.freelist(free.end).end = index;
            self.freelist(free.next).prev = index;
            self.freelist(free.prev).next = index;
        } else {
            // if this is the last element in the block

            self.freelist(free.next).prev = free.prev;
            self.freelist(free.prev).next = free.next;
        }
    }

    unsafe fn insert_slot_into_freelist(&mut self, index: usize) {
        let slot = self.slots.get_unchecked_mut(index);
        match slot.version.mark_empty() {
            Some(next_version) => slot.version = next_version,
            None => {
                // this slot has exhausted it's version counter, so
                // omit it from the freelist and it will never be used again

                // this is works with iteration because iteration always checks
                // if the current slot is vacant, and then accesses `free.end`
                slot.data.free.end = index;
                return
            }
        }

        let is_left_vacant = self.slots.get_unchecked(index.wrapping_sub(1)).is_vacant();
        let is_right_vacant = self.slots.get(index.wrapping_add(1)).map_or(false, Slot::is_vacant);

        match (is_left_vacant, is_right_vacant) {
            (false, false) => {
                // new block

                let head = self.freelist(0);
                let old_head = head.next;
                head.next = index;
                *self.freelist(index) = FreeNode {
                    prev: 0,
                    next: old_head,
                    end: index,
                };
            }
            (false, true) => {
                // prepend

                let front = *self.freelist(index + 1);
                *self.freelist(index) = front;
                self.freelist(front.end).end = index;

                self.freelist(front.next).prev = index;
                self.freelist(front.prev).next = index;
            }
            (true, false) => {
                // append

                let front = self.freelist(index - 1).end;
                self.freelist(index).end = front;
                self.freelist(front).end = index;
            }
            (true, true) => {
                // join

                let next = *self.freelist(index + 1);
                self.freelist(next.prev).next = next.next;
                self.freelist(next.next).prev = next.prev;

                let front = self.freelist(index - 1).end;
                let back = next.end;

                self.freelist(front).end = back;
                self.freelist(back).end = front;
            }
        }
    }

    pub fn get<K: ArenaAccess<I, V>>(&self, key: K) -> Option<&T> { key.get(self) }

    pub fn get_mut<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<&mut T> { key.get_mut(self) }

    pub fn remove_all(&mut self) { self.retain(|_| false) }

    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        let mut i = 0;
        let end = self.slots.len();
        while i != end {
            match unsafe { self.slots.get_unchecked_mut(i).get_mut() } {
                Some(value) => unsafe {
                    if !f(value) {
                        self.remove_unchecked(i);
                    }
                },
                None => i = unsafe { self.freelist(i).end },
            }
            i += 1;
        }
    }

    pub fn values(&self) -> Values<'_, T, V> {
        Values {
            slots: Occupied {
                range: 1..self.num_elements.wrapping_add(1),
                slots: unsafe { self.slots.get_unchecked(1..).iter() },
            },
        }
    }

    pub fn values_mut(&mut self) -> ValuesMut<'_, T, V> {
        ValuesMut {
            slots: OccupiedMut {
                range: 1..self.num_elements.wrapping_add(1),
                slots: unsafe { self.slots.get_unchecked_mut(1..).iter_mut() },
            },
        }
    }

    pub fn into_values(self) -> IntoValues<T, V> {
        let (_, slots) = self.slots.into_raw_parts();
        let mut slots = slots.into_iter();
        core::mem::forget(slots.next());
        IntoValues {
            slots: IntoOccupied {
                range: 1..self.num_elements.wrapping_add(1),
                slots,
            },
        }
    }

    pub fn entries<K: BuildArenaKey<I, V>>(&self) -> Entries<'_, T, I, V, K> {
        Entries {
            slots: Occupied {
                range: 1..self.num_elements.wrapping_add(1),
                slots: unsafe { self.slots.get_unchecked(1..).iter() },
            },
            ident: self.slots.ident(),
            key: PhantomData,
        }
    }

    pub fn entries_mut<K: BuildArenaKey<I, V>>(&mut self) -> EntriesMut<'_, T, I, V, K> {
        let (ident, slots) = self.slots.as_mut_parts();
        EntriesMut {
            slots: OccupiedMut {
                range: 1..self.num_elements.wrapping_add(1),
                slots: unsafe { slots.get_unchecked_mut(1..).iter_mut() },
            },
            ident,
            key: PhantomData,
        }
    }

    pub fn into_entries<K: BuildArenaKey<I, V>>(self) -> IntoEntries<T, I, V, K> {
        let (ident, slots) = self.slots.into_raw_parts();
        let mut slots = slots.into_iter();
        core::mem::forget(slots.next());
        IntoEntries {
            slots: IntoOccupied {
                range: 1..self.num_elements.wrapping_add(1),
                slots,
            },
            ident,
            key: PhantomData,
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

use core::fmt;

use crate::version::{DefaultVersion, Version};

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
                    free: unsafe { self.data.free },
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
        if self.is_occupied() {
            f.debug_struct("Occupied")
                .field("version", &self.version)
                .field("value", unsafe { &*self.data.value })
                .finish()
        } else {
            f.debug_struct("Vacant")
                .field("version", &self.version)
                .field("next", unsafe { &self.data.free.next })
                .field("prev", unsafe { &self.data.free.prev })
                .field("end", unsafe { &self.data.free.end })
                .finish()
        }
    }
}

struct OccupiedBase<I> {
    range: core::ops::Range<usize>,
    slots: I,
}

type Occupied<'a, T, V> = OccupiedBase<core::slice::Iter<'a, Slot<T, V>>>;
type OccupiedMut<'a, T, V> = OccupiedBase<core::slice::IterMut<'a, Slot<T, V>>>;
type IntoOccupied<T, V> = OccupiedBase<std::vec::IntoIter<Slot<T, V>>>;

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

impl<I: Iterator> Iterator for OccupiedBase<I>
where
    I::Item: AsSlot,
{
    type Item = (usize, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let mut index = self.range.next()?;
        let mut slot = self
            .slots
            .next()
            .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });

        loop {
            if slot.as_slot().is_occupied() {
                return Some((index, slot))
            }

            let end = unsafe { slot.as_slot().data.free.end };
            let skip = end.wrapping_sub(index);

            index = self.range.nth(skip)?;
            slot = self
                .slots
                .nth(skip)
                .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (0, self.range.size_hint().1) }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for OccupiedBase<I>
where
    I::Item: AsSlot,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let mut index = self.range.next_back()?;
        let mut slot = self
            .slots
            .next_back()
            .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });

        loop {
            if slot.as_slot().is_occupied() {
                return Some((index, slot))
            }

            let end = unsafe { slot.as_slot().data.free.end };
            let skip = index.wrapping_sub(end);

            index = self.range.nth_back(skip)?;
            slot = self
                .slots
                .nth_back(skip)
                .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });
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
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Keys<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> { self.entries.next_back().map(|(key, _)| key) }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Keys<'_, T, I, V, K> {}

pub struct Values<'a, T, V: Version> {
    slots: Occupied<'a, T, V>,
}

impl<'a, T, V: Version> Iterator for Values<'a, T, V> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|(_, slot)| unsafe { &*slot.data.value }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, V: Version> DoubleEndedIterator for Values<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|(_, slot)| unsafe { &*slot.data.value })
    }
}
impl<T, V: Version> core::iter::FusedIterator for Values<'_, T, V> {}

pub struct ValuesMut<'a, T, V: Version> {
    slots: OccupiedMut<'a, T, V>,
}

impl<'a, T, V: Version> Iterator for ValuesMut<'a, T, V> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|(_, slot)| unsafe { &mut *slot.data.value }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, V: Version> DoubleEndedIterator for ValuesMut<'_, T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|(_, slot)| unsafe { &mut *slot.data.value })
    }
}
impl<T, V: Version> core::iter::FusedIterator for ValuesMut<'_, T, V> {}

pub struct IntoValues<T, V: Version> {
    slots: IntoOccupied<T, V>,
}

impl<T, V: Version> Iterator for IntoValues<T, V> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.next().map(|(_, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, V: Version> DoubleEndedIterator for IntoValues<T, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|(_, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }
}
impl<T, V: Version> core::iter::FusedIterator for IntoValues<T, V> {}

pub struct Entries<'a, T, I, V: Version, K> {
    slots: Occupied<'a, T, V>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for Entries<'a, T, I, V, K> {
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next().map(move |(index, slot)| unsafe {
            (K::new_unchecked(index, slot.version.save(), ident), &*slot.data.value)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for Entries<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next_back().map(move |(index, slot)| unsafe {
            (K::new_unchecked(index, slot.version.save(), ident), &*slot.data.value)
        })
    }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for Entries<'_, T, I, V, K> {}

pub struct EntriesMut<'a, T, I, V: Version, K> {
    slots: OccupiedMut<'a, T, V>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for EntriesMut<'a, T, I, V, K> {
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next().map(move |(index, slot)| unsafe {
            (
                K::new_unchecked(index, slot.version.save(), ident),
                &mut *slot.data.value,
            )
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for EntriesMut<'_, T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots.next_back().map(move |(index, slot)| unsafe {
            (
                K::new_unchecked(index, slot.version.save(), ident),
                &mut *slot.data.value,
            )
        })
    }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for EntriesMut<'_, T, I, V, K> {}

pub struct IntoEntries<T, I, V: Version, K> {
    slots: IntoOccupied<T, V>,
    ident: I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, V: Version, K: BuildArenaKey<I, V>> Iterator for IntoEntries<T, I, V, K> {
    type Item = (K, T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next().map(move |(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut &mut slot.data.value);
            (K::new_unchecked(index, slot.version.save(), ident), value)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, I, V: Version, K: BuildArenaKey<I, V>> DoubleEndedIterator for IntoEntries<T, I, V, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next_back().map(move |(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut &mut slot.data.value);
            (K::new_unchecked(index, slot.version.save(), ident), value)
        })
    }
}
impl<T, I, V: Version, K: BuildArenaKey<I, V>> core::iter::FusedIterator for IntoEntries<T, I, V, K> {}
