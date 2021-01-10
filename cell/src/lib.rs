#![no_std]
#![forbid(missing_docs, clippy::missing_safety_doc)]

//! A shared mutable type that doesn't use guards
//! and gives references directly!
//!
//! `pui_cell` builds atop the foundation of `pui_core`
//! to provide safe shared mutability that can be checked
//! at compile-time (if you want)!

use pui_core::Identifier;

mod get_all_mut;
pub use get_all_mut::GetAllMut;

pub use typsy;
use typsy::{hlist, hlist_pat};

impl<I: ?Sized + Identifier> IdentifierExt for I {}

/// An extension trait that provides functionality to get
/// values out of [`IdCell`]s safely. This trait is automatically
/// implemented for any type that implements [`Identifier`]. So
/// you just need to bring it into scope to use it.
pub trait IdentifierExt: Identifier {
    /// Returns true if this identifier owns the [`IdCell`]
    fn owns<V: ?Sized>(&self, cell: &IdCell<V, Self::Token>) -> bool { self.owns_token(&cell.token) }

    /// Create a new cell that is owned by this identifer
    fn cell<V>(&self, value: V) -> IdCell<V, Self::Token> { IdCell::with_token(value, self.token()) }

    /// Get a shared reference from the [`IdCell`]
    ///
    /// # Panic
    ///
    /// Will panic if self doesn't own the `IdCell`
    fn get<'a, A: ?Sized>(&'a self, a: &'a IdCell<A, Self::Token>) -> &'a A {
        assert!(self.owns(a));
        unsafe { &*a.as_ptr() }
    }

    /// Get a unique reference from the [`IdCell`]
    ///
    /// # Panic
    ///
    /// Will panic if self doesn't own the `IdCell`
    fn get_mut<'a, A: ?Sized>(&'a mut self, a: &'a IdCell<A, Self::Token>) -> &'a mut A {
        assert!(self.owns(a));
        unsafe { &mut *a.as_ptr() }
    }

    /// Get unique references both of the [`IdCell`]s
    ///
    /// # Panic
    ///
    /// Will panic if self doesn't own any of the `IdCell`s or if
    /// either of the two [`IdCell`]s overlap
    fn get_mut2<'a, A: ?Sized, B: ?Sized>(
        &'a mut self,
        a: &'a IdCell<A, Self::Token>,
        b: &'a IdCell<B, Self::Token>,
    ) -> (&'a mut A, &'a mut B) {
        let hlist_pat!(a, b) = self.get_all_mut(hlist!(a, b));
        (a, b)
    }

    /// Get unique references from all three of the [`IdCell`]s
    ///
    /// # Panic
    ///
    /// Will panic if self doesn't own any of the `IdCell`s or if
    /// any of the three [`IdCell`]s overlap
    fn get_mut3<'a, A: ?Sized, B: ?Sized, C: ?Sized>(
        &'a mut self,
        a: &'a IdCell<A, Self::Token>,
        b: &'a IdCell<B, Self::Token>,
        c: &'a IdCell<C, Self::Token>,
    ) -> (&'a mut A, &'a mut B, &'a mut C) {
        let hlist_pat!(a, b, c) = self.get_all_mut(hlist!(a, b, c));
        (a, b, c)
    }

    /// Get unique references from all of the [`IdCell`]s
    ///
    /// # Panic
    ///
    /// Will panic if self doesn't own any of the `IdCell`s or if
    /// any of the three [`IdCell`]s overlap
    fn get_all_mut<'a, L>(&'a mut self, list: L) -> L::Output
    where
        L: GetAllMut<&'a mut Self>,
    {
        self.try_get_all_mut(list).expect("Found overlapping ")
    }

    /// Tries to get unique references from all of the [`IdCell`]s
    /// Returns None if any of the `IdCells` overlap
    ///
    /// # Panic
    ///
    /// Will panic if self doesn't own any of the `IdCell`s
    fn try_get_all_mut<'a, L>(&'a mut self, list: L) -> Option<L::Output>
    where
        L: GetAllMut<&'a mut Self>,
    {
        list.get_all_mut(self)
    }

    /// Swap two `IdCell`s without uninitializing either one
    fn swap<V>(&mut self, a: &IdCell<V, Self::Token>, b: &IdCell<V, Self::Token>) {
        if let Some(hlist_pat!(a, b)) = self.try_get_all_mut(hlist!(a, b)) {
            core::mem::swap(a, b)
        }
    }
}

struct Wrapper<T: ?Sized>(core::cell::UnsafeCell<T>);
unsafe impl<T: Send + Sync> Sync for Wrapper<T> {}

/// A thread-safe shared mutable type that can be
/// allows references into it's interior (unlike `Cell`)
/// without returning guards (unlike `RefCell`, `Mutex`,
/// or `RwLock`).
pub struct IdCell<V: ?Sized, T> {
    /// The token that identifies this `IdCell`
    pub token: T,
    value: Wrapper<V>,
}

impl<V, T: pui_core::Trivial> IdCell<V, T> {
    /// Create a new `IdCell`
    pub fn new(value: V) -> Self { Self::with_token(value, T::INIT) }
}

impl<V, T> IdCell<V, T> {
    /// Create a new `IdCell` with the given token
    pub const fn with_token(value: V, token: T) -> Self {
        Self {
            value: Wrapper(core::cell::UnsafeCell::new(value)),
            token,
        }
    }

    /// Decompose the given `IdCell` into a value-token pair
    pub fn into_raw_parts(self) -> (V, T) { (self.value.0.into_inner(), self.token) }
}

impl<V: ?Sized, T> IdCell<V, T> {
    /// Get a pointer into the interior of the `IdCell`
    pub fn as_ptr(&self) -> *mut V { self.value.0.get() }

    /// Get a mutable reference into the interior of the `IdCell`
    pub fn get_mut(&mut self) -> &mut V { unsafe { &mut *self.value.0.get() } }
}

impl<V: ?Sized, T: pui_core::Trivial> IdCell<V, T> {
    fn assert_trivial() {
        use core::alloc::Layout;
        assert_eq!(Layout::new::<T>(), Layout::new::<()>());
        let _token = T::INIT;
    }

    /// Create a new `IdCell` from a reference to a given type
    ///
    /// Note: this requires the token have the same layout as `()`
    /// and be [`Trivial`](pui_core::Trivial). The [`Trivial`](pui_core::Trivial)
    /// requirement is handled by traits, but if you try and call this with
    /// a token that has a different layout from `()`, `from_mut` this will panic.
    pub fn from_mut(value: &mut V) -> &mut Self {
        Self::assert_trivial();

        unsafe { &mut *(value as *mut V as *mut Self) }
    }
}

impl<V, T: pui_core::Trivial> IdCell<[V], T> {
    /// Convert a cell of a slice to a slice of cells
    ///
    /// Note: this requires the token have the same layout as `()`
    /// and be [`Trivial`](pui_core::Trivial). The [`Trivial`](pui_core::Trivial)
    /// requirement is handled by traits, but if you try and call this with
    /// a token that has a different layout from `()`, `as_slice_of_cells`
    /// this will panic.
    pub fn as_slice_of_cells(&self) -> &[IdCell<V, T>] {
        Self::assert_trivial();
        let ptr = self.as_ptr();
        let ptr = ptr as *const [IdCell<V, T>];
        unsafe { &*ptr }
    }

    /// Convert a cell of a slice to a slice of cells
    ///
    /// Note: this requires the token have the same layout as `()`
    /// and be [`Trivial`](pui_core::Trivial). The [`Trivial`](pui_core::Trivial)
    /// requirement is handled by traits, but if you try and call this with
    /// a token that has a different layout from `()`, `as_slice_of_cells_mut`
    /// this will panic.
    pub fn as_slice_of_cells_mut(&mut self) -> &mut [IdCell<V, T>] {
        Self::assert_trivial();
        let ptr = self.as_ptr();
        let ptr = ptr as *mut [IdCell<V, T>];
        unsafe { &mut *ptr }
    }
}
