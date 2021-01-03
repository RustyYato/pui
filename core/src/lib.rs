#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), feature = "alloc",))]
extern crate alloc as std;

#[doc(hidden)]
pub mod export;
pub mod pool;
pub mod scalar;

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

    fn owns_token(&self, token: &Self::Token) -> bool;

    fn token(&self) -> Self::Token;
}
