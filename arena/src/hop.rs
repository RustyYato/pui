use core::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

pub trait ArenaAccess<I> {
    fn contained_in<T>(&self, arena: &Arena<T, I>) -> bool;

    fn get<'a, T>(&self, arena: &'a Arena<T, I>) -> Option<&'a T>;

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I>) -> Option<&'a mut T>;

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T>;
}

impl<K: ?Sized + ArenaAccess<I>, I> ArenaAccess<I> for &K {
    fn contained_in<T>(&self, arena: &Arena<T, I>) -> bool { K::contained_in(self, arena) }

    fn get<'a, T>(&self, arena: &'a Arena<T, I>) -> Option<&'a T> { K::get(self, arena) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I>) -> Option<&'a mut T> { K::get_mut(self, arena) }

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T> { K::try_remove(self, arena) }
}

pub trait BuildArenaKey<I>: ArenaAccess<I> {
    unsafe fn new_unchecked(index: usize, gen: u32, ident: &I) -> Self;
}

impl<I> ArenaAccess<I> for usize {
    fn contained_in<T>(&self, arena: &Arena<T, I>) -> bool {
        arena.slots.get(*self).map_or(false, |slot| slot.gen & 1 == 0)
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I>) -> Option<&'a T> { arena.slots[self].get() }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I>) -> Option<&'a mut T> { arena.slots[self].get_mut() }

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T> {
        let index = *self;
        let slot = &mut arena.slots[index];

        if slot.is_occupied() {
            unsafe {
                slot.gen = slot.gen.wrapping_add(1);
                let value = ManuallyDrop::take(&mut slot.data.value);
                arena.insert_slot_into_freelist(index);
                Some(value)
            }
        } else {
            None
        }
    }
}

impl<I> BuildArenaKey<I> for usize {
    unsafe fn new_unchecked(index: usize, _: u32, _: &I) -> Self { index }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier> ArenaAccess<I> for pui_vec::Id<I::Token> {
    fn contained_in<T>(&self, arena: &Arena<T, I>) -> bool {
        arena.ident().owns_token(self.token()) && arena.slots[self].gen & 1 == 0
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I>) -> Option<&'a T> { arena.slots[self].get() }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I>) -> Option<&'a mut T> { arena.slots[self].get_mut() }

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T> {
        let index = pui_vec::PuiVecIndex::<I>::slice_index(&self);
        arena.slots[self].remove(index, &mut arena.next)
    }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier> BuildArenaKey<I> for pui_vec::Id<I::Token> {
    unsafe fn new_unchecked(index: usize, _: u32, ident: &I) -> Self {
        pui_vec::Id::new_unchecked(index, ident.token())
    }
}

impl<I> ArenaAccess<I> for Key<usize> {
    fn contained_in<T>(&self, arena: &Arena<T, I>) -> bool {
        let gen = self.gen;
        arena.slots.get(self.id).map_or(false, |slot| slot.gen == gen)
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I>) -> Option<&'a T> { arena.slots[self.id].get_with(self.gen) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I>) -> Option<&'a mut T> {
        arena.slots[self.id].get_mut_with(self.gen)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T> {
        let index = self.id;
        let slot = &mut arena.slots[self.id];

        if slot.gen == self.gen {
            unsafe {
                slot.gen = slot.gen.wrapping_add(1);
                let value = ManuallyDrop::take(&mut slot.data.value);
                arena.insert_slot_into_freelist(index);
                Some(value)
            }
        } else {
            None
        }
    }
}

impl<I> BuildArenaKey<I> for Key<usize> {
    unsafe fn new_unchecked(index: usize, gen: u32, _: &I) -> Self { Key { id: index, gen } }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier> ArenaAccess<I> for Key<pui_vec::Id<I::Token>> {
    fn contained_in<T>(&self, arena: &Arena<T, I>) -> bool {
        let gen = self.gen;
        arena.ident().owns_token(self.id.token())
            && arena.slots.get(self.id.get()).map_or(false, |slot| slot.gen == gen)
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I>) -> Option<&'a T> { arena.slots[&self.id].get_with(self.gen) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I>) -> Option<&'a mut T> {
        arena.slots[&self.id].get_mut_with(self.gen)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T> {
        arena.slots[&self.id].remove_with(self.gen, self.id.get(), &mut arena.next)
    }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier> BuildArenaKey<I> for Key<pui_vec::Id<I::Token>> {
    unsafe fn new_unchecked(index: usize, gen: u32, ident: &I) -> Self {
        Key {
            id: pui_vec::Id::new_unchecked(index, ident.token()),
            gen,
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

struct Slot<T> {
    gen: u32,
    data: Data<T>,
}

#[derive(Clone, Copy)]
pub struct Key<Id> {
    id: Id,
    gen: u32,
}

#[derive(Debug)]
pub struct Arena<T, I = ()> {
    slots: PuiVec<Slot<T>, I>,
    num_elements: usize,
}

pub struct VacantEntry<'a, T, I> {
    arena: &'a mut Arena<T, I>,
    index: usize,
    updated_gen: u32,
    free: FreeNode,
}

impl<T> Slot<T> {
    fn is_occupied(&self) -> bool { self.gen & 1 != 0 }

    fn is_vacant(&self) -> bool { !self.is_occupied() }

    fn get(&self) -> Option<&T> {
        if self.gen & 1 == 0 {
            Some(unsafe { &*self.data.value })
        } else {
            None
        }
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        if self.gen & 1 == 0 {
            Some(unsafe { &mut *self.data.value })
        } else {
            None
        }
    }

    fn get_with(&self, gen: u32) -> Option<&T> {
        if self.gen == gen {
            Some(unsafe { &*self.data.value })
        } else {
            None
        }
    }

    fn get_mut_with(&mut self, gen: u32) -> Option<&mut T> {
        if self.gen == gen {
            Some(unsafe { &mut *self.data.value })
        } else {
            None
        }
    }
}

impl<'a, T, I> VacantEntry<'a, T, I> {
    pub fn insert<K: BuildArenaKey<I>>(self, value: T) -> K {
        unsafe {
            let slot = self.arena.slots.get_unchecked_mut(self.index);
            slot.data = Data {
                value: ManuallyDrop::new(value),
            };
            slot.gen = self.updated_gen;
            self.arena.num_elements += 1;

            self.arena.remove_slot_from_freelist(self.index, self.free);

            K::new_unchecked(self.index, self.updated_gen, self.arena.slots.ident())
        }
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self { Self::with_ident(()) }
}

impl<T, I> Arena<T, I> {
    pub fn with_ident(ident: I) -> Self {
        Self {
            num_elements: 0,
            slots: PuiVec::from_raw_parts(
                std::vec![Slot {
                    gen: 0,
                    data: Data {
                        free: FreeNode {
                            next: 0,
                            prev: 0,
                            end: 0,
                        },
                    },
                }],
                ident,
            ),
        }
    }
}

impl<T, I> Arena<T, I> {
    unsafe fn freelist(&mut self, idx: usize) -> &mut FreeNode { &mut self.slots.get_unchecked_mut(idx).data.free }

    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I> {
        #[cold]
        #[inline(never)]
        unsafe fn allocate_new_node<T, I>(arena: &mut Arena<T, I>, index: usize) {
            arena.slots.push::<usize>(Slot {
                gen: 0,
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

        loop {
            unsafe {
                let head = self.freelist(0).next;

                if head == 0 {
                    // if there are no elements in the freelist

                    let index = self.slots.len();
                    allocate_new_node(self, index);

                    return VacantEntry {
                        arena: self,
                        index,
                        updated_gen: 1,
                        free: FreeNode {
                            next: 0,
                            prev: 0,
                            end: index,
                        },
                    }
                }

                let slot = self.slots.get_unchecked_mut(head);
                let updated_gen = slot.gen | 1;
                let free = slot.data.free;

                if updated_gen == u32::MAX {
                    self.remove_slot_from_freelist_cold(head, free);
                    continue
                }

                return VacantEntry {
                    arena: self,
                    index: head,
                    updated_gen,
                    free,
                }
            }
        }
    }

    pub fn insert<K: BuildArenaKey<I>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    pub fn remove<K: ArenaAccess<I>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove<K: ArenaAccess<I>>(&mut self, key: K) -> Option<T> {
        let value = key.try_remove(self)?;
        self.num_elements -= 1;
        Some(value)
    }

    #[cold]
    #[inline(never)]
    unsafe fn remove_slot_from_freelist_cold(&mut self, index: usize, free: FreeNode) {
        self.remove_slot_from_freelist(index, free)
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

    pub fn get<K: ArenaAccess<I>>(&self, key: K) -> Option<&T> { key.get(self) }

    pub fn get_mut<K: ArenaAccess<I>>(&mut self, key: K) -> Option<&mut T> { key.get_mut(self) }

    pub fn values(&self) -> Values<'_, T> {
        Values {
            slots: Occupied {
                range: 1..self.num_elements.wrapping_add(1),
                slots: unsafe { self.slots.get_unchecked(1..).iter() },
            },
        }
    }

    pub fn values_mut(&mut self) -> ValuesMut<'_, T> {
        ValuesMut {
            slots: OccupiedMut {
                range: 1..self.num_elements.wrapping_add(1),
                slots: unsafe { self.slots.get_unchecked_mut(1..).iter_mut() },
            },
        }
    }

    pub fn into_values(self) -> IntoValues<T> {
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

    pub fn entries<K: BuildArenaKey<I>>(&self) -> Entries<'_, T, I, K> {
        Entries {
            slots: Occupied {
                range: 1..self.num_elements.wrapping_add(1),
                slots: unsafe { self.slots.get_unchecked(1..).iter() },
            },
            ident: self.slots.ident(),
            key: PhantomData,
        }
    }

    pub fn entries_mut<K: BuildArenaKey<I>>(&mut self) -> EntriesMut<'_, T, I, K> {
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

    pub fn into_entries<K: BuildArenaKey<I>>(self) -> IntoEntries<T, I, K> {
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

impl<T, I, K: ArenaAccess<I>> Index<K> for Arena<T, I> {
    type Output = T;

    fn index(&self, key: K) -> &Self::Output { self.get(key).expect("Tried to access `Arena` with a stale `Key`") }
}

impl<T, I, K: ArenaAccess<I>> IndexMut<K> for Arena<T, I> {
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

use core::fmt;

impl<T: fmt::Debug> fmt::Debug for Slot<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_occupied() {
            f.debug_struct("Occupied")
                .field("gen", &self.gen)
                .field("value", unsafe { &*self.data.value })
                .finish()
        } else {
            f.debug_struct("Vacant")
                .field("gen", &self.gen)
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

type Occupied<'a, T> = OccupiedBase<core::slice::Iter<'a, Slot<T>>>;
type OccupiedMut<'a, T> = OccupiedBase<core::slice::IterMut<'a, Slot<T>>>;
type IntoOccupied<T> = OccupiedBase<std::vec::IntoIter<Slot<T>>>;

trait AsSlot {
    type Item;

    fn as_slot(&self) -> &Slot<Self::Item>;
}

impl<T> AsSlot for Slot<T> {
    type Item = T;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item> { self }
}

impl<T> AsSlot for &mut Slot<T> {
    type Item = T;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item> { self }
}

impl<T> AsSlot for &Slot<T> {
    type Item = T;

    #[inline]
    fn as_slot(&self) -> &Slot<Self::Item> { self }
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

pub struct Values<'a, T> {
    slots: Occupied<'a, T>,
}

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|(_, slot)| unsafe { &*slot.data.value }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T> DoubleEndedIterator for Values<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|(_, slot)| unsafe { &*slot.data.value })
    }
}
impl<T> core::iter::FusedIterator for Values<'_, T> {}

pub struct ValuesMut<'a, T> {
    slots: OccupiedMut<'a, T>,
}

impl<'a, T> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> { self.slots.next().map(|(_, slot)| unsafe { &mut *slot.data.value }) }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T> DoubleEndedIterator for ValuesMut<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|(_, slot)| unsafe { &mut *slot.data.value })
    }
}
impl<T> core::iter::FusedIterator for ValuesMut<'_, T> {}

pub struct IntoValues<T> {
    slots: IntoOccupied<T>,
}

impl<T> Iterator for IntoValues<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.next().map(|(_, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T> DoubleEndedIterator for IntoValues<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.next_back().map(|(_, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            ManuallyDrop::take(&mut slot.data.value)
        })
    }
}
impl<T> core::iter::FusedIterator for IntoValues<T> {}

pub struct Entries<'a, T, I, K> {
    slots: Occupied<'a, T>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, K: BuildArenaKey<I>> Iterator for Entries<'a, T, I, K> {
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next()
            .map(move |(index, slot)| unsafe { (K::new_unchecked(index, slot.gen, ident), &*slot.data.value) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, I, K: BuildArenaKey<I>> DoubleEndedIterator for Entries<'_, T, I, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next_back()
            .map(move |(index, slot)| unsafe { (K::new_unchecked(index, slot.gen, ident), &*slot.data.value) })
    }
}
impl<T, I, K: BuildArenaKey<I>> core::iter::FusedIterator for Entries<'_, T, I, K> {}

pub struct EntriesMut<'a, T, I, K> {
    slots: OccupiedMut<'a, T>,
    ident: &'a I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, K: BuildArenaKey<I>> Iterator for EntriesMut<'a, T, I, K> {
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next()
            .map(move |(index, slot)| unsafe { (K::new_unchecked(index, slot.gen, ident), &mut *slot.data.value) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, I, K: BuildArenaKey<I>> DoubleEndedIterator for EntriesMut<'_, T, I, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = self.ident;
        self.slots
            .next_back()
            .map(move |(index, slot)| unsafe { (K::new_unchecked(index, slot.gen, ident), &mut *slot.data.value) })
    }
}
impl<T, I, K: BuildArenaKey<I>> core::iter::FusedIterator for EntriesMut<'_, T, I, K> {}

pub struct IntoEntries<T, I, K> {
    slots: IntoOccupied<T>,
    ident: I,
    key: PhantomData<fn() -> K>,
}

impl<'a, T, I, K: BuildArenaKey<I>> Iterator for IntoEntries<T, I, K> {
    type Item = (K, T);

    fn next(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next().map(move |(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut &mut slot.data.value);
            (K::new_unchecked(index, slot.gen, ident), value)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.slots.size_hint() }
}

impl<T, I, K: BuildArenaKey<I>> DoubleEndedIterator for IntoEntries<T, I, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ident = &self.ident;
        self.slots.next_back().map(move |(index, slot)| unsafe {
            let mut slot = ManuallyDrop::new(slot);
            let value = ManuallyDrop::take(&mut &mut slot.data.value);
            (K::new_unchecked(index, slot.gen, ident), value)
        })
    }
}
impl<T, I, K: BuildArenaKey<I>> core::iter::FusedIterator for IntoEntries<T, I, K> {}
