use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::{
    pool::PoolMut,
    scalar::{OpaqueScalar, ScalarAllocator},
    Identifier, OneShotIdentifier, Token,
};

crate::scalar_allocator! {
    pub struct Global(core::num::NonZeroU64);
}

#[cfg(feature = "std")]
crate::scalar_allocator! {
    pub thread_local struct ThreadLocal(core::num::NonZeroU64);
}

#[derive(Debug)]
pub struct Dynamic<A: ScalarAllocator = Global, P: PoolMut<A> = ()> {
    scalar: A::Scalar,
    pool: P,
    auto: PhantomData<A::AutoTraits>,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct DynamicToken<A: ScalarAllocator = Global> {
    scalar: A::Scalar,
    auto: PhantomData<A::AutoTraits>,
}

impl<A: ScalarAllocator<Scalar = ()>> DynamicToken<A> {
    #[inline]
    pub fn new() -> Self {
        Self {
            scalar: (),
            auto: PhantomData,
        }
    }
}

impl Dynamic {
    #[inline]
    pub fn new() -> Self { Self::with_pool(()) }
}

impl<P: PoolMut<Global>> Dynamic<Global, P> {
    #[inline]
    pub fn with_pool(pool: P) -> Self { Self::with_alloc_and_pool(pool) }
}

impl<A: ScalarAllocator> Dynamic<A> {
    #[inline]
    pub fn with_alloc() -> Self { Self::with_alloc_and_pool(()) }
}

impl<A: ScalarAllocator, P: PoolMut<A>> Dynamic<A, P> {
    #[inline]
    pub fn with_alloc_and_pool(mut pool: P) -> Self {
        Self {
            scalar: pool.remove_mut().map(OpaqueScalar::into_inner).unwrap_or_else(A::alloc),
            pool,
            auto: PhantomData,
        }
    }
}

impl<A: ScalarAllocator, P: PoolMut<A>> Drop for Dynamic<A, P> {
    #[inline]
    fn drop(&mut self) { let _ = self.pool.insert_mut(unsafe { OpaqueScalar::new(self.scalar) }); }
}

impl<A: ScalarAllocator, P: PoolMut<A>> Dynamic<A, P> {
    #[inline]
    fn owns_token(&self, token: DynamicToken<A>) -> bool { self.scalar == token.scalar }

    #[inline]
    fn token(&self) -> DynamicToken<A> {
        DynamicToken {
            scalar: self.scalar,
            auto: PhantomData,
        }
    }
}

unsafe impl<A: ScalarAllocator> Token for DynamicToken<A> {}

unsafe impl<A: ScalarAllocator> OneShotIdentifier for Dynamic<A> {}
unsafe impl<A: ScalarAllocator, P: PoolMut<A>> Identifier for Dynamic<A, P> {
    type Token = DynamicToken<A>;

    #[inline]
    fn owns_token(&self, token: &Self::Token) -> bool { self.owns_token(*token) }

    #[inline]
    fn token(&self) -> Self::Token { self.token() }
}

impl<A: ScalarAllocator<Scalar = ()>> crate::Init for DynamicToken<A> {
    const INIT: Self = Self {
        auto: PhantomData,
        scalar: (),
    };
}

impl<A: ScalarAllocator<Scalar = ()>> crate::Trivial for DynamicToken<A> {}
impl<A: ScalarAllocator> Copy for DynamicToken<A> {}
impl<A: ScalarAllocator> Clone for DynamicToken<A> {
    #[inline]
    fn clone(&self) -> Self { *self }
}

impl<A: ScalarAllocator> Eq for DynamicToken<A> {}
impl<A: ScalarAllocator> PartialEq for DynamicToken<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool { self.scalar == other.scalar }
}

impl<A: ScalarAllocator> Hash for DynamicToken<A> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) { self.scalar.hash(state) }
}

impl<A: ScalarAllocator> PartialOrd for DynamicToken<A> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.scalar.partial_cmp(&other.scalar) }
}

impl<A: ScalarAllocator> Ord for DynamicToken<A> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering { self.scalar.cmp(&other.scalar) }
}
