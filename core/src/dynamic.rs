//! A dynamically created type that is guarnteed to be
//! unique on the given thread or process
//!
//! [`Dynamic`] uses [`ScalarAllocator`] and [`PoolMut`]
//! to ensure that it holds a unique value. This value
//! is cloned into the [`DynamicToken`] and used to
//! verify identity.
//!
//! [`Dynamic`] initially tries to pull a value from [`PoolMut`]
//! if that fails, it then allocates a new value from the
//! [`ScalarAllocator`]. Unless the backing `Scalar` is `()`,
//! then this checks it's identity at runtime. Hence the name,
//! `Dynamic`.
//!
//! If the provided pool is `()`, then no values will be reused,
//! and this allows `Dynamic` to implement [`OneShotIdentifier`].

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU64,
};

use crate::{
    pool::PoolMut,
    scalar::{OpaqueScalar, ScalarAllocator},
    Identifier, OneShotIdentifier, Token,
};

crate::scalar_allocator! {
    /// A global scalar allocator that's backed by a [`NonZeroU64`].
    /// This allows `Option<DynamicToken<Global>>` to be the same size
    /// as [`DynamicToken<Global>`](DynamicToken)
    pub struct Global(NonZeroU64);
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
crate::scalar_allocator! {
    /// A thread-local scalar allocator that's backed by a [`NonZeroU64`]
    /// This allows `Option<DynamicToken<ThreadLocal>>` to be the same size
    /// as [`DynamicToken<ThreadLocal>`](DynamicToken)
    pub thread_local struct ThreadLocal(NonZeroU64);
}

/// A dynamically created type that is guarnteed to be unique on the given thread
/// and if `A::AutoTraits: Send + Sync` on the given process.
///
/// `Dynamic` implements [`OneShotIdentifier`] if `P == ()`, i.e. there is no pool
#[derive(Debug)]
pub struct Dynamic<A: ScalarAllocator = Global, P: PoolMut<A> = ()> {
    scalar: A::Scalar,
    pool: P,
    auto: PhantomData<A::AutoTraits>,
}

/// A token that is recognized by [`Dynamic`]
#[derive(Debug)]
#[repr(transparent)]
pub struct DynamicToken<A: ScalarAllocator = Global> {
    scalar: A::Scalar,
    auto: PhantomData<A::AutoTraits>,
}

impl<A: ScalarAllocator<Scalar = ()>> Default for DynamicToken<A> {
    #[inline]
    fn default() -> Self { Self::NEW }
}

impl<A: ScalarAllocator<Scalar = ()>> DynamicToken<A> {
    /// Create a new token
    pub const NEW: Self = Self {
        scalar: (),
        auto: PhantomData,
    };
}

impl<A: ScalarAllocator> DynamicToken<A> {
    #[inline]
    /// Create a new token
    pub fn new(scalar: A::Scalar) -> Self {
        Self {
            scalar,
            auto: PhantomData,
        }
    }
}

impl Dynamic {
    /// Create a new `Dynamic` using the `Global` `ScalarAllocator`
    #[inline]
    pub fn create() -> Self { Self::with_pool(()) }
}

impl<P: PoolMut<Global>> Dynamic<Global, P> {
    #[inline]
    /// Create a new `Dynamic` using the `Global` `ScalarAllocator` and the given pool
    pub fn with_pool(pool: P) -> Self { Self::with_alloc_and_pool(pool) }
}

impl<A: ScalarAllocator> Dynamic<A> {
    #[inline]
    /// Create a new `Dynamic` using the given `ScalarAllocator`
    pub fn with_alloc() -> Self { Self::with_alloc_and_pool(()) }
}

impl<A: ScalarAllocator, P: PoolMut<A>> Dynamic<A, P> {
    #[inline]
    /// Create a new `Dynamic` using the given `ScalarAllocator` and pool
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
    fn drop(&mut self) { let _ = self.pool.insert_mut(unsafe { OpaqueScalar::new(self.scalar.clone()) }); }
}

impl<A: ScalarAllocator, P: PoolMut<A>> Dynamic<A, P> {
    /// Checks if self created the given [`DynamicToken`]
    #[inline]
    pub fn owns_token(&self, token: &DynamicToken<A>) -> bool { self.scalar == token.scalar }

    /// Creates a [`DynamicToken`]
    #[inline]
    pub fn token(&self) -> DynamicToken<A> {
        DynamicToken {
            scalar: self.scalar.clone(),
            auto: PhantomData,
        }
    }
}

unsafe impl<A: ScalarAllocator> Token for DynamicToken<A> {}

unsafe impl<A: ScalarAllocator> OneShotIdentifier for Dynamic<A> {}
unsafe impl<A: ScalarAllocator, P: PoolMut<A>> Identifier for Dynamic<A, P> {
    type Token = DynamicToken<A>;

    #[inline]
    fn owns_token(&self, token: &Self::Token) -> bool { self.owns_token(token) }

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
impl<A: ScalarAllocator> Copy for DynamicToken<A> where A::Scalar: Copy {}
impl<A: ScalarAllocator> Clone for DynamicToken<A> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            scalar: self.scalar.clone(),
            auto: PhantomData,
        }
    }
}

impl<A: ScalarAllocator> Eq for DynamicToken<A> {}
impl<A: ScalarAllocator> PartialEq for DynamicToken<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool { self.scalar == other.scalar }
}

impl<A: ScalarAllocator> Hash for DynamicToken<A>
where
    A::Scalar: Hash,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) { self.scalar.hash(state) }
}

impl<A: ScalarAllocator> PartialOrd for DynamicToken<A>
where
    A::Scalar: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.scalar.partial_cmp(&other.scalar) }
}

impl<A: ScalarAllocator> Ord for DynamicToken<A>
where
    A::Scalar: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering { self.scalar.cmp(&other.scalar) }
}
