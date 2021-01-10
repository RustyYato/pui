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

/// An empty slot in a hop arena
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
    /// Get the key associated with the `VacantEntry`, this key can be used
    /// once this `VacantEntry` gets filled
    pub fn key<K: BuildArenaKey<I, V>>(&self) -> K {
        unsafe { K::new_unchecked(self.index, self.updated_gen.save(), self.arena.slots.ident()) }
    }

    /// Insert an element into the vacant entry
    pub fn insert<K: BuildArenaKey<I, V>>(self, value: T) -> K {
        unsafe {
            let slot = self.arena.slots.get_unchecked_mut(self.index);
            slot.data = Data {
                value: ManuallyDrop::new(value),
            };
            slot.version = self.updated_gen;
            self.arena.num_elements += 1;
            remove_slot_from_freelist(&mut self.arena.slots, self.index, self.free);

            K::new_unchecked(self.index, self.updated_gen.save(), self.arena.slots.ident())
        }
    }
}

impl<T, I, V: Version> Arena<T, I, V> {
    pub(super) unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        self.num_elements -= 1;
        remove_unchecked(&mut self.slots, index)
    }

    pub(super) unsafe fn delete_unchecked(&mut self, index: usize) {
        struct Fixup<'a, T, V: Version>(&'a mut [Slot<T, V>], usize);

        impl<T, V: Version> Drop for Fixup<'_, T, V> {
            fn drop(&mut self) { unsafe { insert_slot_into_freelist(self.0, self.1) } }
        }

        self.num_elements -= 1;
        let fixup = Fixup(&mut self.slots, index);
        let slot = fixup.0.get_unchecked_mut(index);
        ManuallyDrop::drop(&mut slot.data.value);
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

            freelist(&mut arena.slots, 0).next = index;
        }

        unsafe {
            let head = freelist(&mut self.slots, 0);
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
}

unsafe fn freelist<T, V: Version>(slots: &mut [Slot<T, V>], index: usize) -> &mut FreeNode {
    &mut slots.get_unchecked_mut(index).data.free
}

unsafe fn mu_freelist<T, V: Version>(slots: &mut [Slot<T, V>], index: usize) -> &mut MaybeUninitFreeNode {
    &mut slots.get_unchecked_mut(index).data.mu_free
}

pub(super) unsafe fn remove_unchecked<T, V: Version>(slots: &mut [Slot<T, V>], index: usize) -> T {
    let slot = slots.get_unchecked_mut(index);
    let value = ManuallyDrop::take(&mut slot.data.value);
    insert_slot_into_freelist(slots, index);
    value
}

#[inline(always)]
unsafe fn remove_slot_from_freelist<T, V: Version>(slots: &mut [Slot<T, V>], index: usize, free: MaybeUninitFreeNode) {
    use core::cmp::Ordering;

    if index == 0 {
        core::hint::unreachable_unchecked()
    }

    match free.other_end.assume_init().cmp(&index) {
        Ordering::Equal => {
            // if this is the last element in the block
            mu_freelist(slots, free.next.assume_init()).prev = free.prev;
            mu_freelist(slots, free.prev.assume_init()).next = free.next;
        }
        // if there are more items in the block, and this is the *end* of the block
        // pop this node from the freelist
        Ordering::Less => {
            let other_end = free.other_end.assume_init();
            mu_freelist(slots, other_end).other_end = MaybeUninit::new(index.wrapping_sub(1));
            mu_freelist(slots, index.wrapping_sub(1)).other_end = MaybeUninit::new(other_end)
        }
        // if there are more items in the block, and this is the *start* of the block
        // pop this node from the freelist and rebind the prev and next to point to
        // this node
        Ordering::Greater => {
            let index = index.wrapping_add(1);

            *mu_freelist(slots, index) = free;
            let index = MaybeUninit::new(index);
            mu_freelist(slots, free.other_end.assume_init()).other_end = index;
            mu_freelist(slots, free.next.assume_init()).prev = index;
            mu_freelist(slots, free.prev.assume_init()).next = index;
        }
    };
}

unsafe fn insert_slot_into_freelist<T, V: Version>(slots: &mut [Slot<T, V>], index: usize) {
    let slot = slots.get_unchecked_mut(index);
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

    let is_left_vacant = slots.get_unchecked(index.wrapping_sub(1)).is_vacant();
    let is_right_vacant = slots.get(index.wrapping_add(1)).map_or(false, Slot::is_vacant);

    match (is_left_vacant, is_right_vacant) {
        (false, false) => {
            // new block

            let head = freelist(slots, 0);
            let old_head = head.next;
            head.next = index;
            *mu_freelist(slots, index) = FreeNode {
                prev: 0,
                next: old_head,
                other_end: index,
            }
            .into();
        }
        (false, true) => {
            // prepend

            let front = *freelist(slots, index + 1);
            *mu_freelist(slots, index) = front.into();
            let index = MaybeUninit::new(index);
            mu_freelist(slots, front.other_end).other_end = index;
            mu_freelist(slots, front.next).prev = index;
            mu_freelist(slots, front.prev).next = index;
        }
        (true, false) => {
            // append

            let front = mu_freelist(slots, index - 1).other_end.assume_init();
            mu_freelist(slots, index).other_end = MaybeUninit::new(front);
            mu_freelist(slots, front).other_end = MaybeUninit::new(index);
        }
        (true, true) => {
            // join

            let next = *freelist(slots, index + 1);
            mu_freelist(slots, next.prev).next = MaybeUninit::new(next.next);
            mu_freelist(slots, next.next).prev = MaybeUninit::new(next.prev);

            let front = mu_freelist(slots, index - 1).other_end.assume_init();
            let back = next.other_end;

            mu_freelist(slots, front).other_end = MaybeUninit::new(back);
            mu_freelist(slots, back).other_end = MaybeUninit::new(front);
        }
    }
}
