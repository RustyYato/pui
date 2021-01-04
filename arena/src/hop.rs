use core::mem::ManuallyDrop;

use pui_vec::PuiVec;

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
    next: usize,
}

pub struct VacantEntry<'a, T> {
    arena: &'a mut Arena<T>,
    index: usize,
    updated_gen: u32,
    free: FreeNode,
}

impl<T> Slot<T> {
    fn is_occupied(&self) -> bool { self.gen & 1 != 0 }

    fn is_vacant(&self) -> bool { !self.is_occupied() }
}

impl<'a, T> VacantEntry<'a, T> {
    pub fn insert(self, value: T) -> Key<usize> {
        unsafe {
            let slot = self.arena.slots.get_unchecked_mut(self.index);
            slot.data = Data {
                value: ManuallyDrop::new(value),
            };
            slot.gen = self.updated_gen;

            self.arena.remove_slot_from_freelist(self.index, self.free);

            Key {
                id: self.index,
                gen: self.updated_gen,
            }
        }
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self { Self::with_ident(()) }
}

impl<T, I> Arena<T, I> {
    pub fn with_ident(ident: I) -> Self {
        Self {
            slots: PuiVec::from_raw_parts(
                vec![Slot {
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
            next: 0,
        }
    }
}

impl<T> Arena<T> {
    unsafe fn freelist(&mut self, idx: usize) -> &mut FreeNode { &mut self.slots.get_unchecked_mut(idx).data.free }

    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T> {
        #[cold]
        #[inline(never)]
        unsafe fn allocate_new_node<T>(arena: &mut Arena<T>, index: usize) {
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

    pub fn insert(&mut self, value: T) -> Key<usize> { self.vacant_entry().insert(value) }

    pub fn remove(&mut self, key: Key<usize>) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove(&mut self, key: Key<usize>) -> Option<T> {
        let index = key.id;
        let slot = &mut self.slots[key.id];

        if slot.gen == key.gen {
            unsafe {
                slot.gen = slot.gen.wrapping_add(1);
                let value = ManuallyDrop::take(&mut slot.data.value);
                self.insert_slot_into_freelist(index);
                Some(value)
            }
        } else {
            None
        }
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
