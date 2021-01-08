use core::marker::PhantomData;

use crate::{Identifier, Token};

#[macro_export]
macro_rules! scope {
    ($scope:ident) => {
        let $scope = unsafe { $crate::scoped::Scoped::new_unchecked() };

        let scope = ();
        let assert_unique_scope = $crate::scoped::export::AssertUniqueScope::new(&$scope);
        assert_unique_scope.bind(&scope);
    };
}

#[doc(hidden)]
pub mod export {
    use core::marker::PhantomData;

    pub struct AssertUniqueScope<'a>(PhantomData<super::Invariant<'a>>);

    impl Drop for AssertUniqueScope<'_> {
        #[inline(always)]
        fn drop(&mut self) {}
    }

    impl<'a> AssertUniqueScope<'a> {
        #[inline(always)]
        pub fn new(_: &super::Scoped<'a>) -> Self { Self(PhantomData) }

        #[inline(always)]
        pub fn bind(&self, _: &'a ()) {}
    }
}

struct Invariant<'a>(fn() -> *mut &'a ());

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scoped<'a> {
    invariant: PhantomData<Invariant<'a>>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopedToken<'a> {
    invariant: PhantomData<Invariant<'a>>,
}

impl Scoped<'_> {
    #[doc(hidden)]
    #[inline]
    pub unsafe fn new_unchecked() -> Self { Self { invariant: PhantomData } }

    #[inline]
    pub fn with<R, F: FnOnce(Scoped<'_>) -> R>(f: F) -> R { f(unsafe { Self::new_unchecked() }) }
}

impl ScopedToken<'_> {
    #[inline]
    pub fn new() -> Self { Self { invariant: PhantomData } }
}

impl crate::Init for ScopedToken<'_> {
    const INIT: Self = Self { invariant: PhantomData };
}

impl crate::Trivial for ScopedToken<'_> {}
unsafe impl Token for ScopedToken<'_> {}
unsafe impl crate::OneShotIdentifier for Scoped<'_> {}
unsafe impl<'a> Identifier for Scoped<'a> {
    type Token = ScopedToken<'a>;

    #[inline]
    fn owns_token(&self, _: &Self::Token) -> bool { true }

    #[inline]
    fn token(&self) -> Self::Token { ScopedToken::new() }
}

use core::fmt;

impl fmt::Debug for Scoped<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct("Scoped").finish() }
}

impl fmt::Debug for ScopedToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct("ScopedToken").finish() }
}
