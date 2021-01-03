#![no_std]

use pui_core::Identifier;

mod get_all_mut;
pub use get_all_mut::GetAllMut;

use typsy::{hlist, hlist_pat};

impl<I: ?Sized + Identifier> IdentifierExt for I {}
pub trait IdentifierExt: Identifier {
    fn owns<V: ?Sized>(&self, cell: &IdCell<V, Self::Token>) -> bool { self.owns_token(&cell.token) }

    fn get<'a, A: ?Sized>(&'a self, a: &'a IdCell<A, Self::Token>) -> &'a A {
        assert!(self.owns(a));
        unsafe { &*a.as_ptr() }
    }

    fn get_mut<'a, A: ?Sized>(&'a mut self, a: &'a IdCell<A, Self::Token>) -> &'a mut A {
        assert!(self.owns(a));
        unsafe { &mut *a.as_ptr() }
    }

    fn get_mut2<'a, A: ?Sized, B: ?Sized>(
        &'a mut self,
        a: &'a IdCell<A, Self::Token>,
        b: &'a IdCell<B, Self::Token>,
    ) -> (&'a mut A, &'a mut B) {
        let hlist_pat!(a, b) = self.get_all_mut(hlist!(a, b));
        (a, b)
    }

    fn get_mut3<'a, A: ?Sized, B: ?Sized, C: ?Sized>(
        &'a mut self,
        a: &'a IdCell<A, Self::Token>,
        b: &'a IdCell<B, Self::Token>,
        c: &'a IdCell<C, Self::Token>,
    ) -> (&'a mut A, &'a mut B, &'a mut C) {
        let hlist_pat!(a, b, c) = self.get_all_mut(hlist!(a, b, c));
        (a, b, c)
    }

    fn get_all_mut<'a, L>(&'a mut self, list: L) -> L::Output
    where
        L: GetAllMut<&'a mut Self>,
    {
        self.try_get_all_mut(list).expect("Found overlapping ")
    }

    fn try_get_all_mut<'a, L>(&'a mut self, list: L) -> Option<L::Output>
    where
        L: GetAllMut<&'a mut Self>,
    {
        list.get_all_mut(self)
    }

    fn swap<V>(&self, a: &IdCell<V, Self::Token>, b: &IdCell<V, Self::Token>) {
        assert!(self.owns(a) && self.owns(b));

        let a = a.as_ptr();
        let b = b.as_ptr();
        unsafe { a.swap(b) }
    }
}

struct Wrapper<T: ?Sized>(core::cell::UnsafeCell<T>);
unsafe impl<T: Send + Sync> Sync for Wrapper<T> {}

pub struct IdCell<V: ?Sized, T> {
    pub token: T,
    value: Wrapper<V>,
}

impl<V, T: pui_core::Trivial> IdCell<V, T> {
    pub fn new(value: V) -> Self { Self::with_token(value, T::INIT) }
}

impl<V, T> IdCell<V, T> {
    pub fn with_token(value: V, token: T) -> Self {
        Self {
            value: Wrapper(core::cell::UnsafeCell::new(value)),
            token,
        }
    }
}

impl<V: ?Sized, T> IdCell<V, T> {
    pub fn as_ptr(&self) -> *mut V { self.value.0.get() }

    pub fn get_mut(&mut self) -> &mut V { unsafe { &mut *self.value.0.get() } }
}

impl<V: ?Sized, T: pui_core::Trivial> IdCell<V, T> {
    fn assert_trivial() {
        use core::alloc::Layout;
        assert_eq!(Layout::new::<T>(), Layout::new::<()>());
        let _token = T::INIT;
    }

    pub fn from_mut(value: &mut V) -> &mut Self {
        Self::assert_trivial();

        unsafe { &mut *(value as *mut V as *mut Self) }
    }
}

impl<V, T: pui_core::Trivial> IdCell<[V], T> {
    pub fn as_slice_of_cells(&self) -> &[IdCell<V, T>] {
        Self::assert_trivial();
        let ptr = self.as_ptr();
        let ptr = ptr as *const [IdCell<V, T>];
        unsafe { &*ptr }
    }

    pub fn as_slice_of_cells_mut(&mut self) -> &mut [IdCell<V, T>] {
        Self::assert_trivial();
        let ptr = self.as_ptr();
        let ptr = ptr as *mut [IdCell<V, T>];
        unsafe { &mut *ptr }
    }
}
