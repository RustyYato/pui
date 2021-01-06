use core::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

mod iter_unchecked;
use iter_unchecked::IteratorUnchecked;

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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FreeNode {
    next: usize,
    prev: usize,
    end: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MaybeUninitFreeNode {
    next: MaybeUninit<usize>,
    prev: MaybeUninit<usize>,
    end: MaybeUninit<usize>,
}

impl From<FreeNode> for MaybeUninitFreeNode {
    fn from(FreeNode { next, prev, end }: FreeNode) -> Self {
        Self {
            next: MaybeUninit::new(next),
            prev: MaybeUninit::new(prev),
            end: MaybeUninit::new(end),
        }
    }
}
union Data<T> {
    value: ManuallyDrop<T>,
    free: FreeNode,
    mu_free: MaybeUninitFreeNode,
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
    free: MaybeUninitFreeNode,
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

    unsafe fn freelist(&mut self, idx: usize) -> &mut FreeNode {
        // dbg!(idx);
        &mut self.slots.get_unchecked_mut(idx).data.free
    }

    unsafe fn mu_freelist(&mut self, idx: usize) -> &mut MaybeUninitFreeNode {
        &mut self.slots.get_unchecked_mut(idx).data.mu_free
    }

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
            let head = self.freelist(0);
            let end = head.end;
            let head = head.next;
            let next = [end, head][usize::from(end == 0)];

            if next != 0 {
                let slot = self.slots.get_unchecked_mut(next);
                let updated_gen = slot.version.mark_full();
                let free = slot.data.mu_free;

                VacantEntry {
                    arena: self,
                    index: end,
                    updated_gen,
                    free,
                }
            } else {
                // if there are no elements in the freelist

                let index = self.slots.len();
                allocate_new_node(self, index);

                VacantEntry {
                    arena: self,
                    index,
                    updated_gen: V::EMPTY.mark_full(),
                    free: FreeNode {
                        next: 0,
                        prev: 0,
                        end: index,
                    }
                    .into(),
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

    pub fn contains<K: ArenaAccess<I, V>>(&self, key: K) -> bool { key.contained_in(self) }

    pub unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        let slot = self.slots.get_unchecked_mut(index);
        let value = ManuallyDrop::take(&mut slot.data.value);
        self.insert_slot_into_freelist(index);
        value
    }

    #[inline(always)]
    unsafe fn remove_slot_from_freelist(&mut self, index: usize, free: MaybeUninitFreeNode) {
        use core::cmp::Ordering;

        match free.end.assume_init().cmp(&index) {
            Ordering::Equal => {
                // if this is the last element in the block
                self.mu_freelist(free.next.assume_init()).prev = free.prev;
                self.mu_freelist(free.prev.assume_init()).next = free.next;
                return
            }
            // if there are more items in the block, and this is the *end* of the block
            // pop this node from the freelist
            Ordering::Less => self.mu_freelist(free.end.assume_init()).end = MaybeUninit::new(index.wrapping_sub(1)),
            // if there are more items in the block, and this is the *start* of the block
            // pop this node from the freelist and rebind the prev and next to point to
            // this node
            Ordering::Greater => {
                let index = index.wrapping_add(1);

                *self.mu_freelist(index) = free.into();
                let index = MaybeUninit::new(index);
                self.mu_freelist(free.end.assume_init()).end = index;
                self.mu_freelist(free.next.assume_init()).prev = index;
                self.mu_freelist(free.prev.assume_init()).next = index;
            }
        };
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
                slot.data.mu_free.end = MaybeUninit::new(index);
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
                *self.mu_freelist(index) = FreeNode {
                    prev: 0,
                    next: old_head,
                    end: index,
                }
                .into();
            }
            (false, true) => {
                // prepend

                let front = *self.freelist(index + 1);
                *self.mu_freelist(index) = front.into();
                let index = MaybeUninit::new(index);
                self.mu_freelist(front.end).end = index;
                self.mu_freelist(front.next).prev = index;
                self.mu_freelist(front.prev).next = index;
            }
            (true, false) => {
                // append

                let front = self.mu_freelist(index - 1).end.assume_init();
                self.mu_freelist(index).end = MaybeUninit::new(front);
                self.mu_freelist(front).end = MaybeUninit::new(index);
            }
            (true, true) => {
                // join

                let next = *self.freelist(index + 1);
                self.mu_freelist(next.prev).next = MaybeUninit::new(next.next);
                self.mu_freelist(next.next).prev = MaybeUninit::new(next.prev);

                let front = self.mu_freelist(index - 1).end.assume_init();
                let back = next.end;

                self.mu_freelist(front).end = MaybeUninit::new(back);
                self.mu_freelist(back).end = MaybeUninit::new(front);
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
            let mut debug = f.debug_struct("Vacant");
            debug.field("version", &self.version);

            #[cfg(all(UB_DEBUG, not(miri)))]
            debug
                .field("next", unsafe { &self.data.free.next })
                .field("prev", unsafe { &self.data.free.prev })
                .field("end", unsafe { &self.data.free.end });

            debug.finish()
        }
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
                let skip = slot.data.free.end.wrapping_sub(index).wrapping_add(1);
                self.slots.advance(skip);
            }
            Some(self.slots.next())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (self.len, Some(self.len)) }

    fn count(self) -> usize { self.len }
}

impl<I: iter_unchecked::IteratorUnchecked> DoubleEndedIterator for OccupiedBase<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        unsafe {
            let index = self.slots.index_back();
            let slot = self.slots.peek_back();
            if slot.is_vacant() {
                let skip = index.wrapping_sub(slot.data.free.end);
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
    fn iter_keys_insert_only() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let iter_keys = arena.keys().collect::<Vec<usize>>();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_rev_insert_only() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let iter_keys = arena.keys().rev().collect::<Vec<usize>>();
        ins_keys.reverse();

        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        let iter_keys = arena.keys().collect::<Vec<usize>>();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_rev_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_keys = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_keys.len()).rev().step_by(3) {
            let key = ins_keys.remove(i);
            arena.remove(key);
        }
        ins_keys.reverse();
        let iter_keys = arena.keys().rev().collect::<Vec<usize>>();
        assert_eq!(ins_keys, iter_keys);
    }

    #[test]
    fn iter_keys_with_reinsertion() {
        use std::vec::Vec;
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
        use std::vec::Vec;
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let iter_values = arena.iter().copied().collect::<Vec<_>>();
        assert_eq!(iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_rev_insert_only() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values = arena.iter().copied().rev().collect::<Vec<_>>();
        iter_values.reverse();
        assert_eq!(iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let iter_values = arena.iter().copied().collect::<Vec<_>>();
        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_rev_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values = arena.iter().copied().rev().collect::<Vec<_>>();
        iter_values.reverse();
        assert_eq!(iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_with_reinsertion() {
        use std::vec::Vec;
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
        use std::vec::Vec;
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<_>>();
        assert_eq!(iter_values_mut, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_mut_rev_insert_only() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<_>>();
        iter_values_mut.reverse();
        assert_eq!(iter_values_mut, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn iter_values_mut_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let iter_values_mut = arena.iter_mut().map(|&mut x| x).collect::<Vec<_>>();
        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_mut_rev_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut iter_values_mut = arena.iter_mut().map(|&mut x| x).rev().collect::<Vec<_>>();
        iter_values_mut.reverse();
        assert_eq!(iter_values_mut, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn iter_values_mut_with_reinsertion() {
        use std::vec::Vec;
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
        use std::vec::Vec;
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let into_iter_values = arena.into_iter().collect::<Vec<_>>();
        assert_eq!(into_iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn into_iter_values_rev_insert_only() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let _ = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        let mut into_iter_values = arena.into_iter().rev().collect::<Vec<_>>();
        into_iter_values.reverse();
        assert_eq!(into_iter_values, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn into_iter_values_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let into_iter_values = arena.into_iter().collect::<Vec<_>>();
        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn into_iter_values_rev_with_removal() {
        use std::vec::Vec;
        let mut arena = Arena::new();
        let mut ins_values = (0..10).map(|i| arena.insert(i * 10)).collect::<Vec<usize>>();
        for i in (0..ins_values.len()).rev().step_by(3) {
            let key = ins_values.remove(i);
            arena.remove(key);
        }
        let mut into_iter_values = arena.into_iter().rev().collect::<Vec<_>>();
        into_iter_values.reverse();
        assert_eq!(into_iter_values, [10, 20, 40, 50, 70, 80]);
    }

    #[test]
    fn into_iter_values_with_reinsertion() {
        use std::vec::Vec;
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
