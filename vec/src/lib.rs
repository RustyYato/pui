#![no_std]
#![forbid(missing_docs, clippy::missing_safety_doc)]

//! A append-only vector that uses `pui_core` to brand indicies
//! to allow for unchecked indexing. (Note: `PuiVec` is only
//! append-only if there is an associated `Identifier` attached)
//!
//! # Features
//!
//! `pui` (default) - this hooks into `pui_core` and provides a
//! branded [`Id`] that can be used to elide bound checks.
//!

extern crate alloc as std;

use core::ops::{Deref, DerefMut, Index, IndexMut};
use std::vec::Vec;

#[cfg(feature = "pui-core")]
use pui_core::OneShotIdentifier;

mod pui_vec_index;

pub use pui_vec_index::{BuildPuiVecIndex, PuiVecAccess, PuiVecIndex};

/// A branded index that can be used to elide bounds checks
#[cfg(feature = "pui-core")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Id<T> {
    index: usize,
    token: T,
}

/// An append only `Vec` whitch returns branded indicies that
/// can be used to elide bounds checks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PuiVec<T, I> {
    ident: I,
    vec: Vec<T>,
}

impl<T, I> From<PuiVec<T, I>> for Vec<T> {
    fn from(pui_vec: PuiVec<T, I>) -> Self { pui_vec.vec }
}

#[cfg(feature = "pui-core")]
impl<T> Id<T> {
    /// Create a new branded index
    ///
    /// # Safety
    ///
    /// The given index must be in bounds for the `PuiVec` whose identifier owns
    /// the given token
    pub const unsafe fn new_unchecked(index: usize, token: T) -> Self { Self { index, token } }

    /// Get the index and token from the branded index
    pub fn into_raw_parts(self) -> (usize, T) { (self.index, self.token) }

    /// Returns the index of this [`Id`]
    pub const fn get(&self) -> usize { self.index }

    /// Returns a reference to the token of this [`Id`]
    pub const fn token(&self) -> &T { &self.token }
}

impl<T, I> PuiVec<T, I> {
    /// Creates a new `PuiVec` with the given identifier
    pub const fn new(ident: I) -> Self { Self::from_raw_parts(Vec::new(), ident) }

    /// Creates a new `PuiVec` with the given identifier and `Vec`
    pub const fn from_raw_parts(vec: Vec<T>, ident: I) -> Self { Self { vec, ident } }

    /// Returns a reference to the underlying identifier
    pub const fn ident(&self) -> &I { &self.ident }

    /// Returns true if this `PuiVec` is empty
    pub fn is_empty(&self) -> bool { self.vec.is_empty() }

    /// Returns the length of this `PuiVec`
    pub fn len(&self) -> usize { self.vec.len() }

    /// Returns the capacity of this `PuiVec`
    pub fn capacity(&self) -> usize { self.vec.capacity() }

    /// Reserves at least additional more elements in the `PuiVec`.
    /// i.e. `additional` more elements can be pushed without causing a reallocation
    pub fn reserve(&mut self, additional: usize) { self.vec.reserve(additional) }

    /// Returns a reference to an element or subslice depending on the type of index.
    ///
    /// * If given a position, returns a reference to the element at that position or None if out of bounds.
    /// * If given a range, returns the subslice corresponding to that range, or None if out of bounds.
    /// * If given a Id, returns a reference to the element at that position
    /// * If given a range of Id, returns a the subslice corresponding to that range
    pub fn get<A: PuiVecAccess<T, I>>(&self, index: A) -> Option<&A::Output> { index.get(self) }

    /// Returns a mutable reference to an element or subslice depending on the type of index.
    /// See [`get`](PuiVec::get) for details
    pub fn get_mut<A: PuiVecAccess<T, I>>(&mut self, index: A) -> Option<&mut A::Output> { index.get_mut(self) }

    /// Returns a reference to the identifier and a mutable reference to the underlying slice
    pub fn as_mut_parts(&mut self) -> (&I, &mut [T]) { (&self.ident, &mut self.vec) }

    /// Decomposes `PuiVec` into a it's identifier and it's underling `Vec`
    ///
    /// # Safety
    ///
    /// The identifier can't be used to create a new `PuiVec`
    pub unsafe fn into_raw_parts(self) -> (I, Vec<T>) { (self.ident, self.vec) }
}

// This is safe because `(): !Identifier`, so you can't create a corrosponding `Id`.
// Which means there are is no safe unchecked accesses to the `Vec`
impl<T> PuiVec<T, ()> {
    /// Get a mutable reference to the underling `Vec`
    pub fn vec_mut(&mut self) -> &mut Vec<T> { &mut self.vec }
}

impl<T, I> PuiVec<T, I> {
    /// Appends an element to the back of a collection.
    ///
    /// Returns an [`Id`] or [`usize`]
    pub fn push<Id: BuildPuiVecIndex<I, SliceIndex = usize>>(&mut self, value: T) -> Id {
        let index = self.vec.len();

        self.vec.push(value);

        unsafe { Id::new_unchecked(index, &self.ident) }
    }

    /// Moves all the elements of `other` into `Self`, leaving `other` empty.
    pub fn append(&mut self, vec: &mut Vec<T>) { self.vec.append(vec); }

    /// Clones and appends all elements in a slice to the `PuiVec`.
    ///
    /// Iterates over the slice other, clones each element, and
    /// then appends it to this Vec. The other vector is traversed in-order.
    ///
    /// Note that this function is same as extend except that it
    /// is specialized to work with slices instead. If and when
    /// Rust gets specialization this function will likely be
    /// deprecated (but still available).
    pub fn extend_from_slice(&mut self, slice: &[T])
    where
        T: Clone,
    {
        self.vec.extend_from_slice(slice);
    }
}

// TODO - move `swap`, `split_at`, and `split_at_mut` out to be based on `PuiVecIndex`
#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVec<T, I> {
    /// Returns an iterator over all the ids in the `PuiVec`
    pub fn ids(&self) -> impl ExactSizeIterator<Item = Id<I::Token>> + Clone {
        let token = self.ident.token();
        (0..self.len()).map(move |index| Id {
            index,
            token: token.clone(),
        })
    }

    /// check if the `index` is in bounds, and if it is,
    /// return the corrosponding `Id`
    pub fn parse_id(&self, index: usize) -> Option<Id<I::Token>> {
        if index < self.len() {
            Some(Id {
                index,
                token: self.ident.token(),
            })
        } else {
            None
        }
    }

    /// swap two elements, while eliding bounds checks
    pub fn swap(&mut self, a: Id<I::Token>, b: Id<I::Token>) {
        assert!(self.ident.owns_token(&a.token) && self.ident.owns_token(&b.token));

        let ptr = self.vec.as_mut_ptr();
        unsafe { ptr.add(a.index).swap(ptr.add(b.index)) }
    }

    /// Divides the `PuiVec` into two slices at an index, while eliding bounds checks.
    ///
    /// The first will contain all indices from [0, mid)
    /// (excluding the index mid itself) and the second
    /// will contain all indices from [mid, len)
    /// (excluding the index len itself).
    pub fn split_at(&self, mid: Id<I::Token>) -> (&[T], &[T]) {
        assert!(self.ident.owns_token(&mid.token));
        let len = self.len();
        let ptr = self.vec.as_ptr();
        unsafe {
            (
                core::slice::from_raw_parts(ptr, mid.index),
                core::slice::from_raw_parts(ptr.add(mid.index), len - mid.index),
            )
        }
    }

    /// Divides the `PuiVec` into two slices at an index, while eliding bounds checks.
    ///
    /// The first will contain all indices from [0, mid)
    /// (excluding the index mid itself) and the second
    /// will contain all indices from [mid, len)
    /// (excluding the index len itself).
    pub fn split_at_mut(&mut self, id: Id<I::Token>) -> (&mut [T], &mut [T]) {
        assert!(self.ident.owns_token(&id.token));
        let len = self.len();
        let ptr = self.vec.as_mut_ptr();
        unsafe {
            (
                core::slice::from_raw_parts_mut(ptr, id.index),
                core::slice::from_raw_parts_mut(ptr.add(id.index), len - id.index),
            )
        }
    }
}

impl<T, I> IntoIterator for PuiVec<T, I> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter { self.vec.into_iter() }
}

impl<A, T, I> Extend<A> for PuiVec<T, I>
where
    Vec<T>: Extend<A>,
{
    fn extend<Iter: IntoIterator<Item = A>>(&mut self, iter: Iter) { self.vec.extend(iter) }
}

impl<T, I, A> Index<A> for PuiVec<T, I>
where
    A: PuiVecAccess<T, I>,
{
    type Output = A::Output;

    fn index(&self, index: A) -> &Self::Output { index.index(self) }
}

impl<T, I, A> IndexMut<A> for PuiVec<T, I>
where
    A: PuiVecAccess<T, I>,
{
    fn index_mut(&mut self, index: A) -> &mut Self::Output { index.index_mut(self) }
}

impl<T, I> Deref for PuiVec<T, I> {
    type Target = [T];

    fn deref(&self) -> &Self::Target { &self.vec }
}

impl<T, I> DerefMut for PuiVec<T, I> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.vec }
}
