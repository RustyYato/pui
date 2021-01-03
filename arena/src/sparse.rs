use core::{
    mem::ManuallyDrop,
    ops::{Index, IndexMut},
};

use pui_core::OneShotIdentifier;
use pui_vec::{Id, PuiVec};

union Data<T> {
    value: ManuallyDrop<T>,
    next: usize,
}

struct Slot<T> {
    gen: u32,
    data: Data<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Key<T> {
    index: usize,
    gen: u32,
    token: T,
}

#[derive(Debug)]
pub struct Arena<T, I> {
    slots: PuiVec<Slot<T>, I>,
    next: usize,
}

pub struct VacantEntry<'a, T, I: OneShotIdentifier> {
    key: Key<I::Token>,
    slot: &'a mut Slot<T>,
    next: &'a mut usize,
    new_next: usize,
}

impl<T> Drop for Slot<T> {
    fn drop(&mut self) {
        if self.gen & 1 == 0 {
            unsafe { ManuallyDrop::drop(&mut self.data.value) }
        }
    }
}

impl<T, I> Arena<T, I> {
    pub fn new(ident: I) -> Self {
        Self {
            slots: PuiVec::new(ident),
            next: 0,
        }
    }

    pub fn reserve(&mut self, additional: usize) { self.slots.reserve(additional) }
}

impl<T: pui_core::Token> Key<T> {
    pub fn index(&self) -> Id<T> { unsafe { Id::new_unchecked(self.index, self.token.clone()) } }

    pub fn into_index(self) -> Id<T> { unsafe { Id::new_unchecked(self.index, self.token) } }
}

impl<T, I: pui_core::OneShotIdentifier> VacantEntry<'_, T, I> {
    pub fn key(&self) -> Key<I::Token> { self.key.clone() }

    pub fn insert(self, value: T) -> Key<I::Token> {
        self.slot.data = Data {
            value: ManuallyDrop::new(value),
        };
        self.slot.gen = self.slot.gen.wrapping_add(1);
        *self.next = self.new_next;
        self.key
    }
}

impl<T, I: pui_core::OneShotIdentifier> Arena<T, I> {
    #[inline]
    pub fn parse_index(&self, index: usize) -> Option<Key<I::Token>> {
        let slot = self.slots.get(index)?;
        if slot.gen & 1 == 0 {
            Some(Key {
                index,
                gen: slot.gen,
                token: self.slots.ident().token(),
            })
        } else {
            None
        }
    }

    pub fn vacant_entry(&mut self) -> VacantEntry<'_, T, I> {
        #[cold]
        #[inline(never)]
        pub fn allocate_vacant_slot<T, I: OneShotIdentifier>(this: &mut Arena<T, I>) {
            this.next = this.slots.len();
            this.slots.push(Slot {
                gen: u32::MAX,
                data: Data {
                    next: this.next.wrapping_add(1),
                },
            });
        }

        let len = self.slots.len();
        'find_vacant_slot: loop {
            while len != self.next {
                let slot = unsafe { self.slots.get_unchecked(self.next) };

                if slot.gen != u32::MAX - 2 {
                    break 'find_vacant_slot
                }

                self.next = unsafe { slot.data.next }
            }

            allocate_vacant_slot(self);

            break 'find_vacant_slot
        }

        let token = self.slots.ident().token();
        let slot = unsafe { self.slots.get_unchecked_mut(self.next) };

        let key = Key {
            gen: slot.gen.wrapping_add(1),
            index: self.next,
            token,
        };

        VacantEntry {
            new_next: unsafe { slot.data.next },
            next: &mut self.next,
            slot,
            key,
        }
    }

    pub fn insert(&mut self, value: T) -> Key<I::Token> { self.vacant_entry().insert(value) }

    pub fn remove(&mut self, key: Key<I::Token>) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove(&mut self, key: Key<I::Token>) -> Option<T> {
        let gen = key.gen;
        let key = key.into_index();
        let index = key.get();
        let slot = &mut self.slots[key];

        if slot.gen == gen {
            slot.gen |= 1;
            let value = unsafe { ManuallyDrop::take(&mut slot.data.value) };
            slot.data = Data { next: self.next };
            self.next = index;
            Some(value)
        } else {
            None
        }
    }

    pub fn contains(&self, key: Key<I::Token>) -> bool {
        self.slots.get(key.index()).map_or(false, |slot| slot.gen == key.gen)
    }

    pub fn get(&self, key: Key<I::Token>) -> Option<&T> {
        let slot = &self.slots[key.index()];
        if slot.gen == key.gen {
            Some(unsafe { &slot.data.value })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: Key<I::Token>) -> Option<&mut T> {
        let slot = &mut self.slots[key.index()];
        if slot.gen == key.gen {
            Some(unsafe { &mut slot.data.value })
        } else {
            None
        }
    }
}

impl<T, I: OneShotIdentifier> Index<Key<I::Token>> for Arena<T, I> {
    type Output = T;

    fn index(&self, key: Key<I::Token>) -> &Self::Output {
        self.get(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

impl<T, I: OneShotIdentifier> IndexMut<Key<I::Token>> for Arena<T, I> {
    fn index_mut(&mut self, key: Key<I::Token>) -> &mut Self::Output {
        self.get_mut(key).expect("Tried to access `Arena` with a stale `Key`")
    }
}

impl<T, I: OneShotIdentifier> Extend<T> for Arena<T, I> {
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        iter.for_each(move |value| drop(self.insert(value)));
    }
}

use core::fmt;

impl<T: fmt::Debug> fmt::Debug for Slot<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.gen & 1 == 0 {
            f.debug_struct("Occupied")
                .field("gen", &self.gen)
                .field("value", unsafe { &*self.data.value })
                .finish()
        } else {
            f.debug_struct("Vacant")
                .field("gen", &self.gen)
                .field("next", unsafe { &self.data.next })
                .finish()
        }
    }
}
