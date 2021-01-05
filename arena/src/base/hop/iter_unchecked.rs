use core::{marker::PhantomData, mem::ManuallyDrop, ptr::NonNull};

use crate::version::Version;

use super::Slot;

pub(super) trait IteratorUnchecked {
    type Item;

    type SlotItem;
    type SlotVersion: Version;

    fn len(&self) -> usize;

    unsafe fn peek(&self) -> &Slot<Self::SlotItem, Self::SlotVersion>;

    unsafe fn peek_back(&self) -> &Slot<Self::SlotItem, Self::SlotVersion>;

    unsafe fn next(&mut self) -> Self::Item;

    unsafe fn next_back(&mut self) -> Self::Item;

    unsafe fn advance(&mut self, n: usize);

    unsafe fn advance_back(&mut self, n: usize);

    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate { index: 0, iter: self }
    }
}

pub struct Enumerate<I> {
    index: usize,
    iter: I,
}

impl<I> Enumerate<I> {
    pub fn index(&self) -> usize { self.index }
}

impl<I: IteratorUnchecked> IteratorUnchecked for Enumerate<I> {
    type Item = (usize, I::Item);

    type SlotItem = I::SlotItem;
    type SlotVersion = I::SlotVersion;

    fn len(&self) -> usize { self.iter.len() }

    unsafe fn peek(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { self.iter.peek() }

    unsafe fn peek_back(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { self.iter.peek_back() }

    unsafe fn next(&mut self) -> Self::Item {
        self.index += 1;
        (self.index, self.iter.next())
    }

    unsafe fn next_back(&mut self) -> Self::Item {
        let next = self.iter.next_back();
        (self.iter.len().wrapping_add(self.index), next)
    }

    unsafe fn advance(&mut self, n: usize) {
        self.index += n;
        self.iter.advance(n);
    }

    unsafe fn advance_back(&mut self, n: usize) { self.iter.advance_back(n) }
}

pub(super) struct Iter<'a, T> {
    front: *const T,
    back: *const T,
    lt: PhantomData<&'a [T]>,
}

impl<'a, T> Iter<'a, T> {
    pub fn new(slice: &[T]) -> Self {
        let core::ops::Range {
            start: front,
            end: back,
        } = slice.as_ptr_range();
        Self {
            front,
            back,
            lt: PhantomData,
        }
    }
}

unsafe impl<T: Sync> Send for Iter<'_, T> {}
unsafe impl<T: Sync> Sync for Iter<'_, T> {}

impl<'a, T, V: Version> IteratorUnchecked for Iter<'a, Slot<T, V>> {
    type Item = (V, &'a T);
    type SlotItem = T;
    type SlotVersion = V;

    unsafe fn peek(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { &*self.front }

    unsafe fn peek_back(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { &*self.back }

    fn len(&self) -> usize { unsafe { self.back.offset_from(self.front) as usize } }

    unsafe fn next(&mut self) -> Self::Item {
        let front = self.front;
        self.advance(1);
        ((*front).version, &*(*front).data.value)
    }

    unsafe fn next_back(&mut self) -> Self::Item {
        self.advance_back(1);
        ((*self.back).version, &*(*self.back).data.value)
    }

    unsafe fn advance(&mut self, n: usize) { self.front = self.front.add(n); }

    unsafe fn advance_back(&mut self, n: usize) { self.back = self.back.sub(n); }
}

pub(super) struct IterMut<'a, T> {
    front: *mut T,
    back: *mut T,
    lt: PhantomData<&'a mut [T]>,
}

impl<'a, T> IterMut<'a, T> {
    pub fn new(slice: &mut [T]) -> Self {
        let core::ops::Range {
            start: front,
            end: back,
        } = slice.as_mut_ptr_range();
        Self {
            front,
            back,
            lt: PhantomData,
        }
    }
}

unsafe impl<T: Send> Send for IterMut<'_, T> {}
unsafe impl<T: Sync> Sync for IterMut<'_, T> {}

impl<'a, T, V: Version> IteratorUnchecked for IterMut<'a, Slot<T, V>> {
    type Item = (V, &'a mut T);
    type SlotItem = T;
    type SlotVersion = V;

    unsafe fn peek(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { &*self.front }

    unsafe fn peek_back(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { &*self.back }

    fn len(&self) -> usize { unsafe { self.back.offset_from(self.front) as usize } }

    unsafe fn next(&mut self) -> Self::Item {
        let front = self.front;
        self.advance(1);
        ((*front).version, &mut *(*front).data.value)
    }

    unsafe fn next_back(&mut self) -> Self::Item {
        self.advance_back(1);
        ((*self.back).version, &mut *(*self.back).data.value)
    }

    unsafe fn advance(&mut self, n: usize) { self.front = self.front.add(n); }

    unsafe fn advance_back(&mut self, n: usize) { self.back = self.back.sub(n); }
}

pub(super) struct IntoIter<T> {
    buf: NonNull<T>,
    cap: usize,
    front: *const T,
    back: *const T,
    lt: PhantomData<std::vec::Vec<T>>,
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        struct Dealloc<T>(NonNull<T>, usize);

        impl<T> Drop for Dealloc<T> {
            fn drop(&mut self) {
                unsafe {
                    std::vec::Vec::from_raw_parts(self.0.as_ptr(), 0, self.1);
                }
            }
        }

        unsafe {
            let len = self.back.offset_from(self.front) as usize;
            let _dealloc = Dealloc(self.buf, self.cap);
            core::ptr::slice_from_raw_parts_mut(self.front as *mut T, len).drop_in_place();
        }
    }
}

impl<T> IntoIter<T> {
    pub fn new(vec: std::vec::Vec<T>) -> Self {
        let mut vec = ManuallyDrop::new(vec);
        let len = vec.len();
        let cap = vec.capacity();
        let ptr = vec.as_mut_ptr();

        Self {
            buf: unsafe { NonNull::new_unchecked(ptr) },
            cap,
            front: ptr,
            back: unsafe { ptr.add(len) },
            lt: PhantomData,
        }
    }
}

unsafe impl<T: Send> Send for IntoIter<T> {}
unsafe impl<T: Sync> Sync for IntoIter<T> {}

impl<'a, T, V: Version> IteratorUnchecked for IntoIter<Slot<T, V>> {
    type Item = (V, T);
    type SlotItem = T;
    type SlotVersion = V;

    unsafe fn peek(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { &*self.front }

    unsafe fn peek_back(&self) -> &Slot<Self::SlotItem, Self::SlotVersion> { &*self.back }

    fn len(&self) -> usize { unsafe { self.back.offset_from(self.front) as usize } }

    unsafe fn next(&mut self) -> Self::Item {
        let front = self.front;
        self.advance(1);
        ((*front).version, core::ptr::read(&*(*front).data.value))
    }

    unsafe fn next_back(&mut self) -> Self::Item {
        self.advance_back(1);
        ((*self.back).version, core::ptr::read(&*(*self.back).data.value))
    }

    // skips over vacant blocks, which don't need to be dropped
    unsafe fn advance(&mut self, n: usize) { self.front = self.front.add(n); }

    // skips over vacant blocks, which don't need to be dropped
    unsafe fn advance_back(&mut self, n: usize) { self.back = self.back.sub(n); }
}
