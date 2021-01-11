#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::declare_interior_mutable_const)]
#![forbid(missing_docs, clippy::missing_safety_doc)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! `pui-core` provides process unique identifiers. These identifiers, as the name
//! suggests are unique within the process they reside in. `pui-core` also provides
//! thread-local unique identifiers that are unique within the thread they reside in.
//!
/// These identifiers can be used to enable safe (mostly) compile-time checked
/// shared mutability.
///
/// ```rust
/// use pui::Identifier;
/// use std::cell::UnsafeCell;
///
/// struct Owner<I> {
///     ident: I,
/// }
///
/// struct Handle<H, T: ?Sized> {
///     handle: H,
///     value: UnsafeCell<T>,
/// }
///
/// impl<H, T> Handle<H, T> {
///     pub fn new(handle: H, value: T) -> Self {
///         Self { handle, value: UnsafeCell::new(value) }
///     }
/// }
///
/// impl<I> Owner<I> {
///     pub fn new(ident: I) -> Self {
///         Self { ident }
///     }
/// }
///
/// impl<I: Identifier> Owner<I> {
///     pub fn read<'a, T: ?Sized>(&'a self, handle: &'a Handle<I::Handle, T>) -> &'a T {
///         assert!(self.ident.owns(&handle.handle));
///         
///         // This is safe because `ident` owns the `handle`, which means that `self`
///         // is the only `Owner` that could shared access the underlying value
///         // This is because:
///         //  * the `Owner` owns the `Identifier`
///         //  * when we read/write, we bind the lifetime of `self` and `Handle`
///         //      to the lifetime of the output reference
///         //  * we have shared access to `*self`
///         
///         unsafe { &*handle.value.get() }
///     }
///
///     pub fn write<'a, T: ?Sized>(&'a mut self, handle: &'a Handle<I::Handle, T>) -> &'a mut T {
///         assert!(self.ident.owns(&handle.handle));
///         
///         // This is safe because `ident` owns the `handle`, which means that `self`
///         // is the only `Owner` that could exclusive access the underlying value
///         // This is because:
///         //  * the `Owner` owns the `Identifier`
///         //  * when we read/write, we bind the lifetime of `self` and `Handle`
///         //      to the lifetime of the output reference
///         //  * we have exclusive access to `*self`
///         
///         unsafe { &mut *handle.value.get() }
///     }
/// }
/// ```
///

#[cfg(all(not(feature = "std"), feature = "alloc",))]
extern crate alloc as std;

#[doc(hidden)]
pub mod export;
pub mod pool;
pub mod scalar;

pub mod dynamic;
pub mod scoped;
pub mod ty;

pub(crate) use seal::Seal;
#[forbid(missing_docs)]
mod seal {
    pub trait Seal {}
}

/// A const initializer
pub trait Init {
    /// The initial value of `Self`
    const INIT: Self;
}

/// A type that an [`Identifier`] produces and is owned by an `Identifier`
///
/// If two tokens compare equal, then they should behave identically under
/// `Identifier::owns_token` operation.
///
/// # Safety
///
/// * it should be not possible to change the behavior of `PartialEq::eq`
///   or `Identifier::owns_token` via a shared reference to a `Token`
/// * clones/copies of a token should be equal to each other
pub unsafe trait Token: Clone + Eq {}
/// A [`Token`] that has no safety requirements
pub trait Trivial: Token + Init {}

/// An [`Identifier`] who's tokens are guaranteed to *never* be owned by another
/// `Identifier`, even if this one is dropped
pub unsafe trait OneShotIdentifier: Identifier {}

/// An [`Identifier`] is a process unique identifier
///
/// you are guaranteed that two instances of this identifier will *never* compare equal
/// You can also get a token that this identifier recognizes, which you can use to mark
/// other types as logically owned by the identifier. No other identifier will recognize
/// tokens made be a different identifier while both identifiers are live.
///
/// # Safety
///
/// * `ident.owns(&token)` must return true for any `token` returned
///     from `ident.token()` regardless of when the token was created.
/// * If two tokens compare equal, then `Identifier::owns` must act the
///     same for both of them
///     * i.e. it must return false for both tokens, or it must return
///         true for both tokens
/// * Two instances of `Identifier` must *never* return true for the same
///     token if either the two identifier or the tokens they generate can
///     both exist on the same thread.
pub unsafe trait Identifier {
    /// The tokens that this `Identifier` generates
    type Token: Token;

    #[inline]
    /// Check if this token was created by this identifier
    fn owns_token(&self, token: &Self::Token) -> bool { self.token() == *token }

    /// Create a new token
    fn token(&self) -> Self::Token;
}

unsafe impl<I: ?Sized + Identifier> Identifier for &mut I {
    type Token = I::Token;

    fn owns_token(&self, token: &Self::Token) -> bool { I::owns_token(self, token) }

    fn token(&self) -> Self::Token { I::token(self) }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
unsafe impl<I: ?Sized + Identifier> Identifier for std::boxed::Box<I> {
    type Token = I::Token;

    fn owns_token(&self, token: &Self::Token) -> bool { I::owns_token(self, token) }

    fn token(&self) -> Self::Token { I::token(self) }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
unsafe impl<I: ?Sized + OneShotIdentifier> OneShotIdentifier for std::boxed::Box<I> {}
