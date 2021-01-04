use core::{
    mem::{replace, ManuallyDrop},
    ops::{Index, IndexMut},
};

use pui_vec::PuiVec;

use crate::version::{DefaultVersion, Version};

pub(crate) struct TrustedIndex(usize);

impl TrustedIndex {
    #[inline]
    pub unsafe fn new(index: usize) -> Self { Self(index) }
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

pub trait BuildArenaKey<I, V: Version>: ArenaAccess<I, V> {
    unsafe fn new_unchecked(index: usize, save: V::Save, ident: &I) -> Self;
}

impl<I, V: Version> ArenaAccess<I, V> for usize {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        arena.slots.get(*self).map_or(false, |slot| slot.version.is_full())
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[self].get() }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> { arena.slots[self].get_mut() }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        arena.slots[self].remove(*self, &mut arena.next)
    }
}

impl<I, V: Version> BuildArenaKey<I, V> for usize {
    unsafe fn new_unchecked(index: usize, _: V::Save, _: &I) -> Self { index }
}

impl<I, V: Version> ArenaAccess<I, V> for TrustedIndex {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        unsafe { arena.slots.get_unchecked(self.0).version.is_full() }
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> {
        unsafe { arena.slots.get_unchecked(self.0) }.get()
    }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> {
        unsafe { arena.slots.get_unchecked_mut(self.0) }.get_mut()
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        unsafe { arena.slots.get_unchecked_mut(self.0) }.remove(self.0, &mut arena.next)
    }
}

#[cfg(feature = "pui-core")]
impl<I: pui_core::OneShotIdentifier, V: Version> ArenaAccess<I, V> for pui_vec::Id<I::Token> {
    fn contained_in<T>(&self, arena: &Arena<T, I, V>) -> bool {
        arena.ident().owns_token(self.token()) && arena.slots[self].version.is_full()
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[self].get() }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> { arena.slots[self].get_mut() }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        let index = pui_vec::PuiVecIndex::<I>::slice_index(&self);
        arena.slots[self].remove(index, &mut arena.next)
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
        let version = self.version;
        arena
            .slots
            .get(self.id)
            .map_or(false, |slot| slot.version.equals_saved(version))
    }

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[self.id].get_with(self.version) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> {
        arena.slots[self.id].get_mut_with(self.version)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        arena.slots[self.id].remove_with(self.version, self.id, &mut arena.next)
    }
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

    fn get<'a, T>(&self, arena: &'a Arena<T, I, V>) -> Option<&'a T> { arena.slots[&self.id].get_with(self.version) }

    fn get_mut<'a, T>(&self, arena: &'a mut Arena<T, I, V>) -> Option<&'a mut T> {
        arena.slots[&self.id].get_mut_with(self.version)
    }

    fn try_remove<T>(&self, arena: &mut Arena<T, I, V>) -> Option<T> {
        arena.slots[&self.id].remove_with(self.version, self.id.get(), &mut arena.next)
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

union Data<T> {
    value: ManuallyDrop<T>,
    next: usize,
}

struct Slot<T, V: Version> {
    version: V,
    data: Data<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Key<Id, V> {
    id: Id,
    version: V,
}

#[derive(Debug)]
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

    fn remove(&mut self, index: usize, next: &mut usize) -> Option<T> {
        if self.version.is_full() {
            Some(unsafe { self.remove_unchecked(index, next) })
        } else {
            None
        }
    }

    fn remove_with(&mut self, saved: V::Save, index: usize, next: &mut usize) -> Option<T> {
        if self.version.equals_saved(saved) {
            Some(unsafe { self.remove_unchecked(index, next) })
        } else {
            None
        }
    }

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
    pub fn new() -> Self {
        Self {
            slots: PuiVec::new(()),
            next: 0,
        }
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

    pub fn try_remove<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<T> { key.try_remove(self) }

    pub fn contains<K: ArenaAccess<I, V>>(&self, key: K) -> bool { key.contained_in(self) }

    pub fn get<K: ArenaAccess<I, V>>(&self, key: K) -> Option<&T> { key.get(self) }

    pub fn get_mut<K: ArenaAccess<I, V>>(&mut self, key: K) -> Option<&mut T> { key.get_mut(self) }

    pub fn keys<K: BuildArenaKey<I, V>>(&self) -> impl '_ + Iterator<Item = K> {
        let ident = self.ident();
        self.slots.iter().enumerate().filter_map(move |(index, slot)| {
            if slot.version.is_full() {
                Some(unsafe { K::new_unchecked(index, slot.version.save(), ident) })
            } else {
                None
            }
        })
    }

    pub fn values(&self) -> impl '_ + Iterator<Item = &T> {
        self.slots.iter().filter_map(move |slot| {
            if slot.version.is_full() {
                Some(unsafe { &*slot.data.value })
            } else {
                None
            }
        })
    }

    pub fn values_mut(&mut self) -> impl '_ + Iterator<Item = &mut T> {
        self.slots.iter_mut().filter_map(move |slot| {
            if slot.version.is_full() {
                Some(unsafe { &mut *slot.data.value })
            } else {
                None
            }
        })
    }

    pub fn into_values(self) -> impl Iterator<Item = T> {
        self.slots.into_iter().filter_map(move |slot| {
            let mut slot = ManuallyDrop::new(slot);
            if slot.version.is_full() {
                Some(unsafe { ManuallyDrop::take(&mut slot.data.value) })
            } else {
                None
            }
        })
    }

    pub fn entries<K: BuildArenaKey<I, V>>(&self) -> impl '_ + Iterator<Item = (K, &T)> {
        let ident = self.ident();
        self.slots.iter().enumerate().filter_map(move |(index, slot)| {
            if slot.version.is_full() {
                Some(unsafe { (K::new_unchecked(index, slot.version.save(), ident), &*slot.data.value) })
            } else {
                None
            }
        })
    }

    pub fn entries_mut<K: BuildArenaKey<I, V>>(&mut self) -> impl '_ + Iterator<Item = (K, &mut T)> {
        let (ident, slots) = self.slots.as_mut_parts();
        slots.iter_mut().enumerate().filter_map(move |(index, slot)| {
            if slot.version.is_full() {
                Some(unsafe {
                    (
                        K::new_unchecked(index, slot.version.save(), ident),
                        &mut *slot.data.value,
                    )
                })
            } else {
                None
            }
        })
    }

    pub fn into_entries<K: BuildArenaKey<I, V>>(self) -> impl Iterator<Item = (K, T)> {
        let (ident, slots) = self.slots.into_raw_parts();
        slots.into_iter().enumerate().filter_map(move |(index, slot)| {
            let mut slot = ManuallyDrop::new(slot);
            if slot.version.is_full() {
                Some(unsafe {
                    (
                        K::new_unchecked(index, slot.version.save(), &ident),
                        ManuallyDrop::take(&mut slot.data.value),
                    )
                })
            } else {
                None
            }
        })
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
