#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::declare_interior_mutable_const)]
// FIXME - remove this when documenting all features
#![allow(clippy::missing_safety_doc)]

//! `pui-core` provides process unique identifiers. These identifiers, as the name
//! suggests are unique within the process they reside in. `pui-core` also provides
//! thread-local unique identifiers that are unique within the thread they reside in.
//!
//!

#[cfg(all(not(feature = "std"), feature = "alloc",))]
extern crate alloc as std;

#[doc(hidden)]
pub mod export;
pub mod pool;
pub mod scalar;

pub mod anonomous;
pub mod dynamic;
pub mod scoped;

pub(crate) use seal::Seal;
#[forbid(missing_docs)]
mod seal {
    pub trait Seal {}
}

pub trait Init {
    const INIT: Self;
}

pub unsafe trait Token: Clone + Eq {}
pub trait Trivial: Token + Init {}

pub unsafe trait OneShotIdentifier: Identifier {}
pub unsafe trait Identifier {
    type Token: Token;

    #[inline]
    fn owns_token(&self, token: &Self::Token) -> bool { self.token() == *token }

    fn token(&self) -> Self::Token;
}
