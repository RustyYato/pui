use core::{
    mem::{replace, ManuallyDrop},
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

pub(crate) struct TrustedIndex(usize);

impl TrustedIndex {
    #[inline]
    pub unsafe fn new(index: usize) -> Self { Self(index) }
}

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

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T> { arena.slots[self].remove(*self, &mut arena.next) }
}

impl<I> ArenaAccess<I> for TrustedIndex {
    fn contained_in<T>(&self, arena: &Arena<T, I>) -> bool { unsafe { arena.slots.get_unchecked(self.0).gen & 1 == 0 } }

    fn get<'a, T>(&self, arena: &'a Arena<T, I>) -> Option<&'a T> { unsafe { arena.slots.get_unchecked(self.0) }.get() }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I>) -> Option<&'a mut T> {
        unsafe { arena.slots.get_unchecked_mut(self.0) }.get_mut()
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I>) -> Option<T> {
        unsafe { arena.slots.get_unchecked_mut(self.0) }.remove(self.0, &mut arena.next)
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
        arena.slots[self.id].remove_with(self.gen, self.id, &mut arena.next)
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

union Data<T> {
    value: ManuallyDrop<T>,
    next: usize,
}

struct Slot<T> {
    gen: u32,
    data: Data<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Key<Id> {
    id: Id,
    gen: u32,
}

#[derive(Debug)]
pub struct Arena<T, I = ()> {
    slots: PuiVec<Slot<T>, I>,
    next: usize,
}

pub struct VacantEntry<'a, T, K> {
    key: K,
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

impl<T> Slot<T> {
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

    fn remove(&mut self, index: usize, next: &mut usize) -> Option<T> { self.remove_with(self.gen, index, next) }

    fn remove_with(&mut self, gen: u32, index: usize, next: &mut usize) -> Option<T> {
        if self.gen == gen {
            self.gen |= 1;
            let value = unsafe { ManuallyDrop::take(&mut self.data.value) };
            self.data = Data {
                next: replace(next, index),
            };
            Some(value)
        } else {
            None
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            slots: PuiVec::new(()),
            next: 0,
        }
    }
}

impl<T, I> Arena<T, I> {
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

impl<Id> Key<Id> {
    pub fn id(&self) -> &Id { &self.id }

    pub fn generation(&self) -> u32 { self.gen }
}

impl<T, K> VacantEntry<'_, T, K> {
    pub fn key(&self) -> &K { &self.key }

    pub fn insert(self, value: T) -> K {
        self.slot.data = Data {
            value: ManuallyDrop::new(value),
        };
        self.slot.gen = self.slot.gen.wrapping_add(1);
        *self.next = self.new_next;
        self.key
    }
}

impl<T, I> Arena<T, I> {
    #[inline]
    pub fn parse_key<K: BuildArenaKey<I>>(&self, index: usize) -> Option<K> {
        let slot = self.slots.get(index)?;
        if slot.gen & 1 == 0 {
            Some(unsafe { K::new_unchecked(index, slot.gen, self.slots.ident()) })
        } else {
            None
        }
    }
}

impl<T, I> Arena<T, I> {
    pub fn vacant_entry<K: BuildArenaKey<I>>(&mut self) -> VacantEntry<'_, T, K> {
        #[cold]
        #[inline(never)]
        pub fn allocate_vacant_slot<T, I>(this: &mut Arena<T, I>) {
            this.next = this.slots.len();
            let _: usize = this.slots.push(Slot {
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

        let slot = unsafe { self.slots.get_unchecked(self.next) };
        let key = unsafe { K::new_unchecked(self.next, slot.gen.wrapping_add(1), self.slots.ident()) };
        let slot = unsafe { self.slots.get_unchecked_mut(self.next) };

        VacantEntry {
            new_next: unsafe { slot.data.next },
            next: &mut self.next,
            slot,
            key,
        }
    }

    pub fn insert<K: BuildArenaKey<I>>(&mut self, value: T) -> K { self.vacant_entry().insert(value) }

    pub fn remove<K: ArenaAccess<I>>(&mut self, key: K) -> T {
        self.try_remove(key)
            .expect("Could not remove form an `Arena` using a stale `Key`")
    }

    pub fn try_remove<K: ArenaAccess<I>>(&mut self, key: K) -> Option<T> { key.try_remove(self) }

    pub fn contains<K: ArenaAccess<I>>(&self, key: K) -> bool { key.contained_in(self) }

    pub fn get<K: ArenaAccess<I>>(&self, key: K) -> Option<&T> { key.get(self) }

    pub fn get_mut<K: ArenaAccess<I>>(&mut self, key: K) -> Option<&mut T> { key.get_mut(self) }

    pub fn keys<K: BuildArenaKey<I>>(&self) -> impl '_ + Iterator<Item = K> {
        let ident = self.ident();
        self.slots.iter().enumerate().filter_map(move |(index, slot)| {
            if slot.gen & 1 == 0 {
                Some(unsafe { K::new_unchecked(index, slot.gen, ident) })
            } else {
                None
            }
        })
    }

    pub fn values(&self) -> impl '_ + Iterator<Item = &T> {
        self.slots.iter().filter_map(move |slot| {
            if slot.gen & 1 == 0 {
                Some(unsafe { &*slot.data.value })
            } else {
                None
            }
        })
    }

    pub fn values_mut(&mut self) -> impl '_ + Iterator<Item = &mut T> {
        self.slots.iter_mut().filter_map(move |slot| {
            if slot.gen & 1 == 0 {
                Some(unsafe { &mut *slot.data.value })
            } else {
                None
            }
        })
    }

    pub fn into_values(self) -> impl Iterator<Item = T> {
        self.slots.into_iter().filter_map(move |slot| {
            let mut slot = ManuallyDrop::new(slot);
            if slot.gen & 1 == 0 {
                Some(unsafe { ManuallyDrop::take(&mut slot.data.value) })
            } else {
                None
            }
        })
    }

    pub fn entries<K: BuildArenaKey<I>>(&self) -> impl '_ + Iterator<Item = (K, &T)> {
        let ident = self.ident();
        self.slots.iter().enumerate().filter_map(move |(index, slot)| {
            if slot.gen & 1 == 0 {
                Some(unsafe { (K::new_unchecked(index, slot.gen, ident), &*slot.data.value) })
            } else {
                None
            }
        })
    }

    pub fn entries_mut<K: BuildArenaKey<I>>(&mut self) -> impl '_ + Iterator<Item = (K, &mut T)> {
        let (ident, slots) = self.slots.as_mut_parts();
        slots.iter_mut().enumerate().filter_map(move |(index, slot)| {
            if slot.gen & 1 == 0 {
                Some(unsafe { (K::new_unchecked(index, slot.gen, ident), &mut *slot.data.value) })
            } else {
                None
            }
        })
    }

    pub fn into_entries<K: BuildArenaKey<I>>(self) -> impl Iterator<Item = (K, T)> {
        let (ident, slots) = self.slots.into_raw_parts();
        slots.into_iter().enumerate().filter_map(move |(index, slot)| {
            let mut slot = ManuallyDrop::new(slot);
            if slot.gen & 1 == 0 {
                Some(unsafe {
                    (
                        K::new_unchecked(index, slot.gen, &ident),
                        ManuallyDrop::take(&mut slot.data.value),
                    )
                })
            } else {
                None
            }
        })
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

impl<T, I> Extend<T> for Arena<T, I> {
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        iter.for_each(move |value| drop::<Key<usize>>(self.vacant_entry().insert(value)));
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
