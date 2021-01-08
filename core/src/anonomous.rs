use core::marker::PhantomData;
use std::hash::{Hash, Hasher};

use crate::{Identifier, Token};

struct Invariant<T>(fn() -> *mut T);
pub struct Anonomous<T> {
    invariant: PhantomData<Invariant<T>>,
}

pub struct AnonomousToken<T> {
    invariant: PhantomData<Invariant<T>>,
}

#[macro_export]
macro_rules! make_anonomous {
    () => {
        unsafe { struct AnonInner; $crate::anonomous::Anonomous::new(&AnonInner) }
    };
    (
        $(#[$meta:meta])*
        $v:vis let $name:ident;
    ) => {
        $(#[$meta])*
        #[allow(non_camel_case_types)]
        $v struct $name {}
        let $name = unsafe { $crate::anonomous::Anonomous::new(&$name {}) };
    };
}

impl<T> Default for AnonomousToken<T> {
    fn default() -> Self { Self { invariant: PhantomData } }
}

impl<T> Anonomous<T> {
    #[doc(hidden)]
    pub const unsafe fn new(_: &T) -> Self { Self { invariant: PhantomData } }

    #[inline]
    pub const fn owns_token(&self, _: &AnonomousToken<T>) -> bool { true }

    #[inline]
    pub const fn token(&self) -> AnonomousToken<T> { AnonomousToken::new() }
}

impl<T> AnonomousToken<T> {
    pub const fn new() -> Self { Self { invariant: PhantomData } }
}

unsafe impl<T> Token for AnonomousToken<T> {}
unsafe impl<T> Identifier for Anonomous<T> {
    type Token = AnonomousToken<T>;

    #[inline]
    fn owns_token(&self, _: &Self::Token) -> bool { true }

    #[inline]
    fn token(&self) -> Self::Token { AnonomousToken::new() }
}

impl<T> Copy for AnonomousToken<T> {}
impl<T> Clone for AnonomousToken<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> Eq for AnonomousToken<T> {}
impl<T> PartialEq for AnonomousToken<T> {
    fn eq(&self, _: &Self) -> bool { true }
}

impl<T> PartialOrd for AnonomousToken<T> {
    fn partial_cmp(&self, _: &Self) -> Option<core::cmp::Ordering> { Some(core::cmp::Ordering::Equal) }
}
impl<T> Ord for AnonomousToken<T> {
    fn cmp(&self, _: &Self) -> core::cmp::Ordering { core::cmp::Ordering::Equal }
}

impl<T> Hash for AnonomousToken<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { ().hash(state) }
}
