use core::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

use crate::{Id, PuiVec};

use pui_core::OneShotIdentifier;

use seal::Seal;
#[forbid(missing_docs)]
mod seal {
    pub trait Seal: Sized {}
}

pub trait PuiVecIndex<T, I>: Seal {
    type SliceIndex: core::slice::SliceIndex<[T], Output = Self::Output>;
    type Output: ?Sized;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool;

    fn slice_index(self) -> Self::SliceIndex;

    unsafe fn get_unchecked(self, vec: &PuiVec<T, I>) -> &Self::Output { vec.vec.get_unchecked(self.slice_index()) }

    unsafe fn get_unchecked_mut(self, vec: &mut PuiVec<T, I>) -> &mut Self::Output {
        vec.vec.get_unchecked_mut(self.slice_index())
    }

    fn get(self, vec: &PuiVec<T, I>) -> Option<&Self::Output> {
        if self.contained_in(vec) {
            Some(unsafe { self.get_unchecked(vec) })
        } else {
            None
        }
    }

    fn get_mut(self, vec: &mut PuiVec<T, I>) -> Option<&mut Self::Output> {
        if self.contained_in(vec) {
            Some(unsafe { self.get_unchecked_mut(vec) })
        } else {
            None
        }
    }

    fn index(self, vec: &PuiVec<T, I>) -> &Self::Output {
        if self.contained_in(vec) {
            unsafe { self.get_unchecked(vec) }
        } else {
            &vec.vec[self.slice_index()]
        }
    }

    fn index_mut(self, vec: &mut PuiVec<T, I>) -> &mut Self::Output {
        if self.contained_in(vec) {
            unsafe { self.get_unchecked_mut(vec) }
        } else {
            &mut vec.vec[self.slice_index()]
        }
    }
}

#[cold]
#[inline(never)]
fn not_owned() -> ! { panic!("Tried to use an id that isn't owned by the `PuiVec`") }

impl<T> Seal for Id<T> {}
impl<T, I: OneShotIdentifier> PuiVecIndex<T, I> for Id<I::Token> {
    type Output = T;
    type SliceIndex = usize;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(self) -> Self::SliceIndex { self.index }
}

impl Seal for RangeFull {}
impl<T, I: OneShotIdentifier> PuiVecIndex<T, I> for RangeFull {
    type Output = [T];
    type SliceIndex = RangeFull;

    fn contained_in(&self, _: &PuiVec<T, I>) -> bool { true }

    fn slice_index(self) -> Self::SliceIndex { self }
}

impl<T> Seal for RangeTo<Id<T>> {}
impl<T, I: OneShotIdentifier> PuiVecIndex<T, I> for RangeTo<Id<I::Token>> {
    type Output = [T];
    type SliceIndex = RangeTo<usize>;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.end.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(self) -> Self::SliceIndex { ..self.end.index }
}

impl<T> Seal for RangeFrom<Id<T>> {}
impl<T, I: OneShotIdentifier> PuiVecIndex<T, I> for RangeFrom<Id<I::Token>> {
    type Output = [T];
    type SliceIndex = RangeFrom<usize>;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.start.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(self) -> Self::SliceIndex { self.start.index.. }
}

impl<T> Seal for RangeToInclusive<Id<T>> {}
impl<T, I: OneShotIdentifier> PuiVecIndex<T, I> for RangeToInclusive<Id<I::Token>> {
    type Output = [T];
    type SliceIndex = RangeToInclusive<usize>;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.end.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(self) -> Self::SliceIndex { ..=self.end.index }
}

impl<T> Seal for Range<Id<T>> {}
impl<T, I: OneShotIdentifier> PuiVecIndex<T, I> for Range<Id<I::Token>> {
    type Output = [T];
    type SliceIndex = Range<usize>;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.start.token) && vec.ident.owns_token(&self.end.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(self) -> Self::SliceIndex { self.start.index..self.end.index }
}

impl<T> Seal for RangeInclusive<Id<T>> {}
impl<T, I: OneShotIdentifier> PuiVecIndex<T, I> for RangeInclusive<Id<I::Token>> {
    type Output = [T];
    type SliceIndex = RangeInclusive<usize>;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.start().token) && vec.ident.owns_token(&self.end().token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(self) -> Self::SliceIndex { self.start().index..=self.end().index }
}

impl Seal for usize {}
impl<T, I> PuiVecIndex<T, I> for usize {
    type Output = T;
    type SliceIndex = Self;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(*self).is_some() }

    fn slice_index(self) -> Self::SliceIndex { self }
}

impl Seal for RangeTo<usize> {}
impl<T, I> PuiVecIndex<T, I> for RangeTo<usize> {
    type Output = [T];
    type SliceIndex = Self;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(self) -> Self::SliceIndex { self }
}

impl Seal for RangeFrom<usize> {}
impl<T, I> PuiVecIndex<T, I> for RangeFrom<usize> {
    type Output = [T];
    type SliceIndex = Self;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(self) -> Self::SliceIndex { self }
}

impl Seal for RangeToInclusive<usize> {}
impl<T, I> PuiVecIndex<T, I> for RangeToInclusive<usize> {
    type Output = [T];
    type SliceIndex = Self;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(self) -> Self::SliceIndex { self }
}

impl Seal for Range<usize> {}
impl<T, I> PuiVecIndex<T, I> for Range<usize> {
    type Output = [T];
    type SliceIndex = Self;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(self) -> Self::SliceIndex { self }
}

impl Seal for RangeInclusive<usize> {}
impl<T, I> PuiVecIndex<T, I> for RangeInclusive<usize> {
    type Output = [T];
    type SliceIndex = Self;

    fn contained_in(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(self) -> Self::SliceIndex { self }
}
