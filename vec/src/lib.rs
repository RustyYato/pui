#![no_std]

extern crate alloc as std;

use core::ops::{Deref, DerefMut, Index, IndexMut};
use std::vec::Vec;

#[cfg(feature = "pui-core")]
use pui_core::OneShotIdentifier;

mod pui_vec_index;

pub use pui_vec_index::{PuiVecAccess, PuiVecIndex, BuildPuiVecIndex};

#[cfg(feature = "pui-core")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id<T> {
    index: usize,
    token: T,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PuiVec<T, I> {
    ident: I,
    vec: Vec<T>,
}

#[cfg(feature = "pui-core")]
impl<T> Id<T> {
    pub const unsafe fn new_unchecked(index: usize, token: T) -> Self { Self { index, token } }

    pub fn into_raw_parts(self) -> (usize, T) { (self.index, self.token) }

    pub const fn get(&self) -> usize { self.index }

    pub const fn token(&self) -> &T { &self.token }
}

impl<T, I> PuiVec<T, I> {
    pub const fn new(ident: I) -> Self { Self::from_raw_parts(Vec::new(), ident) }

    pub const fn from_raw_parts(vec: Vec<T>, ident: I) -> Self { Self { vec, ident } }

    pub const fn ident(&self) -> &I { &self.ident }

    pub fn len(&self) -> usize { self.vec.len() }

    pub fn capacity(&self) -> usize { self.vec.capacity() }

    pub fn reserve(&mut self, additional: usize) { self.vec.reserve(additional) }

    pub fn get<A: PuiVecAccess<T, I>>(&self, index: A) -> Option<&A::Output> { index.get(self) }

    pub fn get_mut<A: PuiVecAccess<T, I>>(&mut self, index: A) -> Option<&mut A::Output> { index.get_mut(self) }
}

impl<T, I> PuiVec<T, I> {
    pub fn push<Id: BuildPuiVecIndex<I, SliceIndex = usize>>(&mut self, value: T) -> Id {
        let index = self.vec.len();

        self.vec.push(value);

        unsafe { Id::new_unchecked(index, &self.ident) }
    }

    pub fn append(&mut self, vec: &mut Vec<T>) { self.vec.append(vec); }

    pub fn extend_from_slice(&mut self, slice: &[T])
    where
        T: Clone,
    {
        self.vec.extend_from_slice(slice);
    }
}

#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVec<T, I> {
    pub fn ids(&self) -> impl ExactSizeIterator<Item = Id<I::Token>> + Clone {
        let token = self.ident.token();
        (0..self.len()).map(move |index| Id {
            index,
            token: token.clone(),
        })
    }

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

    pub fn swap(&mut self, a: Id<I::Token>, b: Id<I::Token>) {
        assert!(self.ident.owns_token(&a.token) && self.ident.owns_token(&b.token));

        let ptr = self.vec.as_mut_ptr();
        unsafe { ptr.add(a.index).swap(ptr.add(b.index)) }
    }

    pub fn split_at(&self, id: Id<I::Token>) -> (&[T], &[T]) {
        assert!(self.ident.owns_token(&id.token));
        let len = self.len();
        let ptr = self.vec.as_ptr();
        unsafe {
            (
                core::slice::from_raw_parts(ptr, id.index),
                core::slice::from_raw_parts(ptr.add(id.index), len - id.index),
            )
        }
    }

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
