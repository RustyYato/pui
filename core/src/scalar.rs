//! [`Scalar`] defined types that can be used to back the [`ScalarAllocator`]s
//!
//! `ScalarAllocator`s are types that produce a sequence of `Scalar`s with no
//! duplicates in the sequence

use core::{
    cell::Cell,
    cmp,
    hash::{Hash, Hasher},
    num,
    sync::atomic::{Ordering::*, *},
};

use radium::Radium;

macro_rules! inc {
    (fn inc($self:ident) -> Option<Self> { $($block:tt)* }) => {
        #[doc(hidden)]
        const LOCAL_INIT: Self::Local = Self::Local::new(0);
        #[doc(hidden)]
        const ATOMIC_INIT: Self::Atomic = Self::Atomic::new(0);

        #[inline]
        #[doc(hidden)]
        fn inc_local($self: &Self::Local) -> Option<Self> { $($block)* }
        #[doc(hidden)]
        fn inc_atomic($self: &Self::Atomic) -> Option<Self> { $($block)* }
    };
}

/// Simple types that can be produced from [`ScalarAllocator`], these types
/// are known to have stable [`Clone`] and [`Eq`] implementations that allow
/// them to be used in the implementation of [`Token`](crate::Token)
pub trait Scalar: crate::Seal + Copy + Ord + Hash {
    #[doc(hidden)]
    type Local;
    #[doc(hidden)]
    type Atomic;

    #[doc(hidden)]
    const LOCAL_INIT: Self::Local;
    #[doc(hidden)]
    const ATOMIC_INIT: Self::Atomic;

    #[doc(hidden)]
    fn inc_local(local: &Self::Local) -> Option<Self>;
    #[doc(hidden)]
    fn inc_atomic(local: &Self::Atomic) -> Option<Self>;
}

/// A opaque type that allows the implementation of [`Pool`](crate::pool::Pool)/
/// [`PoolMut`](crate::pool::PoolMut) to be safe
pub struct OpaqueScalar<S: ScalarAllocator>(S::Scalar);

impl<S: ScalarAllocator> OpaqueScalar<S> {
    /// Create a new [`OpaqueScalar`]
    ///
    /// # Safety
    ///
    /// * You must have goteen this scalar from `OpaqueScalar::into_inner`
    /// * You must create at most 1 `OpaqueScalar` from this `scalar`
    pub unsafe fn new(scalar: S::Scalar) -> Self { Self(scalar) }

    /// Get the underlying scalar from `OpaqueScalar`
    pub fn into_inner(self) -> S::Scalar { self.0 }
}

/// A type that produces a sequence of unique [`Scalar`]s
///
/// # Safety
///
/// * `AutoTraits` should not be `Send` or `Sync` if the sequence
///    is only guaranteed to be unique wihtin a given thread.
/// * `alloc` should never produces two `Scalar`s that compare equal
///    to each other
pub unsafe trait ScalarAllocator {
    /// The types in the sequence
    type Scalar: Scalar;
    /// This type's autotraits restrictions that will
    /// be applied to both `Dynamic` and `DynamicToken`
    type AutoTraits;

    /// The next item in the sequence
    ///
    /// # Panic
    ///
    /// If the sequence is exhausted, `alloc` may panic
    fn alloc() -> Self::Scalar;
}

impl<A: ScalarAllocator> Eq for OpaqueScalar<A> {}
impl<A: ScalarAllocator> PartialEq for OpaqueScalar<A> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

impl<A: ScalarAllocator> Hash for OpaqueScalar<A> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state) }
}

impl<A: ScalarAllocator> PartialOrd for OpaqueScalar<A> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> { self.0.partial_cmp(&other.0) }
}

impl<A: ScalarAllocator> Ord for OpaqueScalar<A> {
    fn cmp(&self, other: &Self) -> cmp::Ordering { self.0.cmp(&other.0) }
}

impl crate::Seal for () {}
impl Scalar for () {
    #[doc(hidden)]
    type Local = Cell<bool>;
    #[doc(hidden)]
    type Atomic = AtomicBool;

    #[doc(hidden)]
    const LOCAL_INIT: Self::Local = Cell::new(false);
    #[doc(hidden)]
    const ATOMIC_INIT: Self::Atomic = AtomicBool::new(false);

    #[doc(hidden)]
    #[inline]
    fn inc_local(local: &Self::Local) -> Option<Self> {
        match local.replace(false) {
            false => Some(()),
            true => None,
        }
    }

    #[doc(hidden)]
    #[inline]
    fn inc_atomic(local: &Self::Atomic) -> Option<Self> {
        match local.swap(false, Relaxed) {
            false => Some(()),
            true => None,
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __scalar_allocator {
    (@create $name:ident) => {
        impl $name {
            /// Create a new `Dynamic<Self>` that implements `OneShotIdentifier`
            pub fn oneshot() -> $crate::dynamic::Dynamic<Self> { $crate::dynamic::Dynamic::with_alloc() }

            /// Create a new `Dynamic<Self>` with the given pool
            pub fn with_pool<P: $crate::pool::PoolMut<Self>>(pool: P) -> $crate::dynamic::Dynamic<Self, P> {
                $crate::dynamic::Dynamic::with_alloc_and_pool(pool)
            }
        }
    };
    (@create type $name:ident) => {
        impl $name {
            pub fn oneshot() -> $crate::dynamic::Dynamic<Self> { $crate::dynamic::Dynamic::with_alloc() }

            pub fn reuse() -> $crate::dynamic::Dynamic<Self, Self> { Self::with_pool(Self) }

            pub fn with_pool<P: $crate::pool::PoolMut<Self>>(pool: P) -> $crate::dynamic::Dynamic<Self, P> {
                $crate::dynamic::Dynamic::with_alloc_and_pool(pool)
            }
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident($scalar:ty);
    ) => {
        $(#[$meta])*
        $v struct $name;

        unsafe impl $crate::scalar::ScalarAllocator for $name {
            type Scalar = $scalar;
            type AutoTraits = ();

            fn alloc() -> Self::Scalar {
                static __SCALAR_ALLOCATOR: <$scalar as $crate::scalar::Scalar>::Atomic = <$scalar as $crate::scalar::Scalar>::ATOMIC_INIT;

                $crate::scalar::Scalar::inc_atomic(&__SCALAR_ALLOCATOR)
                    .expect(concat!(
                        "Could not allocate more scalars from ",
                        stringify!($name),
                    ))
            }
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis thread_local struct $name:ident($scalar:ty);
    ) => {
        $(#[$meta])*
        $v struct $name;

        unsafe impl $crate::scalar::ScalarAllocator for $name {
            type Scalar = $scalar;
            type AutoTraits = $crate::export::NoSendSync;

            fn alloc() -> Self::Scalar {
                $crate::export::thread_local! {
                    static __SCALAR_ALLOCATOR: <$scalar as $crate::scalar::Scalar>::Local = <$scalar as $crate::scalar::Scalar>::LOCAL_INIT;
                }

                __SCALAR_ALLOCATOR.with(|scalar| {
                    $crate::scalar::Scalar::inc_local(scalar)
                }).expect(concat!(
                    "Could not allocate more scalars from ",
                    stringify!($name),
                ))
            }
        }
    };
}

/// Construct a new [`ScalarAllocator`]
///
/// For example:
/// ```rust
/// pui_core::scalar_allocator! {
///     /// Your scalar allocator
///     struct Foo;
/// }
///
/// pui_core::scalar_allocator! {
///     #[derive(Debug)]
///     pub struct Bar(u8);
/// }
///
/// let foo = Foo;
/// let bar = Bar;
/// ```
///
/// `Foo: ScalarAllocator`, and because it is guaratneed to
/// produce one scalar, it also implements `Pool` with a `Flag`
///
/// You can also prefix `struct` with `thread_local` to get a `ScalarAllocator`
/// that is only produces unique scalars on within a given thread
///
/// For example:
/// ```rust
/// pui_core::scalar_allocator! {
///     thread_local struct Foo;
/// }
///
/// pui_core::scalar_allocator! {
///     thread_local struct Bar(u8);
/// }
///
/// let foo = Foo;
/// let bar = Bar;
/// ```
#[macro_export]
macro_rules! scalar_allocator {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident;
    ) => {
        $crate::__scalar_allocator! {
            $(#[$meta])*
            $v struct $name(());
        }

        $crate::__scalar_allocator! {
            @create type $name
        }

        $crate::__global_pool! {
            $name($crate::pool::Flag<$name>)
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis thread_local struct $name:ident;
    ) => {
        $crate::__scalar_allocator! {
            $(#[$meta])*
            $v thread_local struct $name(());
        }

        $crate::__scalar_allocator! {
            @create type $name
        }

        $crate::__global_pool! {
            thread_local $name($crate::export::LocalFlag<$name>)
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident($scalar:ty);
    ) => {
        $crate::__scalar_allocator! {
            $(#[$meta])*
            $v struct $name($scalar);
        }

        $crate::__scalar_allocator! {
            @create $name
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis thread_local struct $name:ident($scalar:ty);
    ) => {
        $crate::__scalar_allocator! {
            $(#[$meta])*
            $v thread_local struct $name($scalar);
        }

        $crate::__scalar_allocator! {
            @create $name
        }
    };
}

macro_rules! norm_prim {
    ($($prim:ty => $atomic:ty, $nonzero:ty,)*) => {$(
        impl crate::Seal for $prim {}
        impl Scalar for $prim {
            #[doc(hidden)]
            type Local = Cell<$prim>;
            #[doc(hidden)]
            type Atomic = $atomic;

            inc! {
                fn inc(this) -> Option<Self> {
                    let mut value = this.load(Relaxed);

                    loop {
                        let next = value.checked_add(1)?;

                        if let Err(current) = this.compare_exchange_weak(value, next, Acquire, Relaxed) {
                            value = current
                        } else {
                            return Some(value)
                        }
                    }
                }
            }
        }

        impl crate::Seal for $nonzero {}
        impl Scalar for $nonzero {
            #[doc(hidden)]
            type Local = Cell<$prim>;
            #[doc(hidden)]
            type Atomic = $atomic;

            inc! {
                fn inc(this) -> Option<Self> {
                    let mut value = this.load(Relaxed);

                    loop {
                        let next = value.checked_add(1)?;

                        if let Err(current) = this.compare_exchange_weak(value, next, Acquire, Relaxed) {
                            value = current
                        } else {
                            return <$nonzero>::new(value.wrapping_add(1))
                        }
                    }
                }
            }
        }
    )*};
}

norm_prim! {
    u8 => AtomicU8, num::NonZeroU8,
    u16 => AtomicU16, num::NonZeroU16,
    u32 => AtomicU32, num::NonZeroU32,
    u64 => AtomicU64, num::NonZeroU64,
    usize => AtomicUsize, num::NonZeroUsize,
}
