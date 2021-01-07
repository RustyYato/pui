use core::{
    fmt,
    mem::{ManuallyDrop, MaybeUninit},
};

use super::{Arena, BuildArenaKey};
use crate::version::{DefaultVersion, Version};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FreeNode {
    next: usize,
    prev: usize,
    other_end: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MaybeUninitFreeNode {
    next: MaybeUninit<usize>,
    prev: MaybeUninit<usize>,
    other_end: MaybeUninit<usize>,
}

impl From<FreeNode> for MaybeUninitFreeNode {
    fn from(FreeNode { next, prev, other_end }: FreeNode) -> Self {
        Self {
            next: MaybeUninit::new(next),
            prev: MaybeUninit::new(prev),
            other_end: MaybeUninit::new(other_end),
        }
    }
}

union Data<T> {
    value: ManuallyDrop<T>,
    free: FreeNode,
    mu_free: MaybeUninitFreeNode,
}

pub(super) struct Slot<T, V: Version> {
    version: V,
    data: Data<T>,
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
                .field("end", unsafe { &self.data.free.other_end });

            debug.finish()
        }
    }
}

impl<T, V: Version> Slot<T, V> {
    pub(super) const SENTINEL: Self = Slot {
        version: V::EMPTY,
        data: Data {
            free: FreeNode {
                next: 0,
                prev: 0,
                other_end: 0,
            },
        },
    };

    pub(super) fn parse_key<I, K: BuildArenaKey<I, V>>(&self, index: usize, ident: &I) -> Option<K> {
        if self.version.is_full() {
            Some(unsafe { K::new_unchecked(index, self.version.save(), ident) })
        } else {
            None
        }
    }

    pub(super) fn version(&self) -> V { self.version }

    pub(super) unsafe fn get_unchecked(&self) -> &T { &*self.data.value }

    pub(super) unsafe fn get_mut_unchecked(&mut self) -> &mut T { &mut *self.data.value }

    pub(super) unsafe fn take_unchecked(&mut self) -> T { ManuallyDrop::take(&mut self.data.value) }

    pub(super) unsafe fn other_end(&self) -> usize { self.data.free.other_end }

    pub(super) fn is_occupied(&self) -> bool { self.version.is_full() }

    pub(super) fn is_vacant(&self) -> bool { self.version.is_empty() }
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

impl<T, I, V: Version> Arena<T, I, V> {
    unsafe fn freelist(&mut self, idx: usize) -> &mut FreeNode { &mut self.slots.get_unchecked_mut(idx).data.free }

    unsafe fn mu_freelist(&mut self, idx: usize) -> &mut MaybeUninitFreeNode {
        &mut self.slots.get_unchecked_mut(idx).data.mu_free
    }

    pub(super) unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        self.num_elements -= 1;
        let slot = self.slots.get_unchecked_mut(index);
        let value = ManuallyDrop::take(&mut slot.data.value);
        self.insert_slot_into_freelist(index);
        value
    }

    pub(super) fn __vacant_entry(&mut self) -> VacantEntry<'_, T, I, V> {
        #[cold]
        #[inline(never)]
        unsafe fn allocate_new_node<T, I, V: Version>(arena: &mut Arena<T, I, V>, index: usize) {
            arena.slots.push::<usize>(Slot {
                version: V::EMPTY,
                data: Data {
                    free: FreeNode {
                        next: 0,
                        prev: 0,
                        other_end: index,
                    },
                },
            });

            arena.freelist(0).next = index;
        }

        unsafe {
            let head = self.freelist(0);
            let end = head.other_end;
            let head = head.next;
            let next = [end, head][usize::from(end == 0)];

            if next != 0 {
                let slot = self.slots.get_unchecked_mut(next);
                let updated_gen = slot.version.mark_full();
                let free = slot.data.mu_free;

                VacantEntry {
                    arena: self,
                    index: next,
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
                        other_end: index,
                    }
                    .into(),
                }
            }
        }
    }

    #[inline(always)]
    unsafe fn remove_slot_from_freelist(&mut self, index: usize, free: MaybeUninitFreeNode) {
        use core::cmp::Ordering;

        if index == 0 {
            core::hint::unreachable_unchecked()
        }

        match free.other_end.assume_init().cmp(&index) {
            Ordering::Equal => {
                // if this is the last element in the block
                self.mu_freelist(free.next.assume_init()).prev = free.prev;
                self.mu_freelist(free.prev.assume_init()).next = free.next;
                return
            }
            // if there are more items in the block, and this is the *end* of the block
            // pop this node from the freelist
            Ordering::Less => {
                self.mu_freelist(free.other_end.assume_init()).other_end = MaybeUninit::new(index.wrapping_sub(1))
            }
            // if there are more items in the block, and this is the *start* of the block
            // pop this node from the freelist and rebind the prev and next to point to
            // this node
            Ordering::Greater => {
                let index = index.wrapping_add(1);

                *self.mu_freelist(index) = free.into();
                let index = MaybeUninit::new(index);
                self.mu_freelist(free.other_end.assume_init()).other_end = index;
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
                // if the current slot is vacant, and then accesses `free.other_end`
                slot.data.mu_free.other_end = MaybeUninit::new(index);
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
                    other_end: index,
                }
                .into();
            }
            (false, true) => {
                // prepend

                let front = *self.freelist(index + 1);
                *self.mu_freelist(index) = front.into();
                let index = MaybeUninit::new(index);
                self.mu_freelist(front.other_end).other_end = index;
                self.mu_freelist(front.next).prev = index;
                self.mu_freelist(front.prev).next = index;
            }
            (true, false) => {
                // append

                let front = self.mu_freelist(index - 1).other_end.assume_init();
                self.mu_freelist(index).other_end = MaybeUninit::new(front);
                self.mu_freelist(front).other_end = MaybeUninit::new(index);
            }
            (true, true) => {
                // join

                let next = *self.freelist(index + 1);
                self.mu_freelist(next.prev).next = MaybeUninit::new(next.next);
                self.mu_freelist(next.next).prev = MaybeUninit::new(next.prev);

                let front = self.mu_freelist(index - 1).other_end.assume_init();
                let back = next.other_end;

                self.mu_freelist(front).other_end = MaybeUninit::new(back);
                self.mu_freelist(back).other_end = MaybeUninit::new(front);
            }
        }
    }
}
