use core::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

#[cfg(feature = "pui-core")]
use crate::Id;
use crate::PuiVec;

#[cfg(feature = "pui-core")]
use pui_core::OneShotIdentifier;

use seal::Seal;
#[forbid(missing_docs)]
mod seal {
    pub trait Seal: Sized {}
}

#[cold]
#[inline(never)]
fn index_fail() -> ! { panic!() }

pub trait PuiVecIndex<I>: Seal {
    type SliceIndex;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool;

    fn slice_index(&self) -> Self::SliceIndex;
}

pub trait BuildPuiVecIndex<I>: PuiVecIndex<I> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, ident: &I) -> Self;
}

pub trait PuiVecAccess<T, I>: PuiVecIndex<I> {
    type Output: ?Sized;

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output;

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output;

    fn get<'a>(&self, vec: &'a PuiVec<T, I>) -> Option<&'a Self::Output> {
        if self.contained_in(vec) {
            Some(unsafe { self.get_unchecked(vec) })
        } else {
            None
        }
    }

    fn get_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> Option<&'a mut Self::Output> {
        if self.contained_in(vec) {
            Some(unsafe { self.get_unchecked_mut(vec) })
        } else {
            None
        }
    }

    fn index<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        if self.contained_in(vec) {
            unsafe { self.get_unchecked(vec) }
        } else {
            index_fail()
        }
    }

    fn index_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        if self.contained_in(vec) {
            unsafe { self.get_unchecked_mut(vec) }
        } else {
            index_fail()
        }
    }
}

impl<Pi: ?Sized + Seal> Seal for &Pi {}
impl<Pi: ?Sized + PuiVecIndex<I>, I> PuiVecIndex<I> for &Pi {
    type SliceIndex = Pi::SliceIndex;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { Pi::contained_in(self, vec) }

    fn slice_index(&self) -> Self::SliceIndex { Pi::slice_index(self) }
}

impl<Pi: ?Sized + PuiVecAccess<T, I>, I, T> PuiVecAccess<T, I> for &Pi {
    type Output = Pi::Output;

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output { Pi::get_unchecked(self, vec) }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        Pi::get_unchecked_mut(self, vec)
    }
}

impl<Pi: ?Sized + Seal> Seal for &mut Pi {}
impl<Pi: ?Sized + PuiVecIndex<I>, I> PuiVecIndex<I> for &mut Pi {
    type SliceIndex = Pi::SliceIndex;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { Pi::contained_in(self, vec) }

    fn slice_index(&self) -> Self::SliceIndex { Pi::slice_index(self) }
}

impl<Pi: ?Sized + PuiVecAccess<T, I>, I, T> PuiVecAccess<T, I> for &mut Pi {
    type Output = Pi::Output;

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output { Pi::get_unchecked(self, vec) }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        Pi::get_unchecked_mut(self, vec)
    }
}

#[cold]
#[inline(never)]
#[cfg(feature = "pui-core")]
fn not_owned() -> ! { panic!("Tried to use an id that isn't owned by the `PuiVec`") }

#[cfg(feature = "pui-core")]
impl<T> Seal for Id<T> {}
#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> PuiVecIndex<I> for Id<I::Token> {
    type SliceIndex = usize;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(&self) -> Self::SliceIndex { self.index }
}

#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> BuildPuiVecIndex<I> for Id<I::Token> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, ident: &I) -> Self {
        Id {
            index: slice_index,
            token: ident.token(),
        }
    }
}

#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVecAccess<T, I> for Id<I::Token> {
    type Output = T;

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.get_unchecked(PuiVecIndex::<I>::slice_index(self))
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.get_unchecked_mut(PuiVecIndex::<I>::slice_index(self))
    }
}

#[cfg(feature = "pui-core")]
impl<T> Seal for RangeTo<Id<T>> {}
#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> PuiVecIndex<I> for RangeTo<Id<I::Token>> {
    type SliceIndex = RangeTo<usize>;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.end.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(&self) -> Self::SliceIndex { ..self.end.index }
}

#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> BuildPuiVecIndex<I> for RangeTo<Id<I::Token>> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, ident: &I) -> Self {
        ..Id {
            index: slice_index.end,
            token: ident.token(),
        }
    }
}

#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVecAccess<T, I> for RangeTo<Id<I::Token>> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.get_unchecked(PuiVecIndex::<I>::slice_index(self))
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.get_unchecked_mut(PuiVecIndex::<I>::slice_index(self))
    }
}

#[cfg(feature = "pui-core")]
impl<T> Seal for RangeFrom<Id<T>> {}
#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> PuiVecIndex<I> for RangeFrom<Id<I::Token>> {
    type SliceIndex = RangeFrom<usize>;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.start.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(&self) -> Self::SliceIndex { self.start.index.. }
}

#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> BuildPuiVecIndex<I> for RangeFrom<Id<I::Token>> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, ident: &I) -> Self {
        Id {
            index: slice_index.start,
            token: ident.token(),
        }..
    }
}

#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVecAccess<T, I> for RangeFrom<Id<I::Token>> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.get_unchecked(PuiVecIndex::<I>::slice_index(self))
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.get_unchecked_mut(PuiVecIndex::<I>::slice_index(self))
    }
}

#[cfg(feature = "pui-core")]
impl<T> Seal for RangeToInclusive<Id<T>> {}
#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> PuiVecIndex<I> for RangeToInclusive<Id<I::Token>> {
    type SliceIndex = RangeToInclusive<usize>;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.end.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(&self) -> Self::SliceIndex { ..=self.end.index }
}

#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> BuildPuiVecIndex<I> for RangeToInclusive<Id<I::Token>> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, ident: &I) -> Self {
        ..=Id {
            index: slice_index.end,
            token: ident.token(),
        }
    }
}

#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVecAccess<T, I> for RangeToInclusive<Id<I::Token>> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.get_unchecked(PuiVecIndex::<I>::slice_index(self))
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.get_unchecked_mut(PuiVecIndex::<I>::slice_index(self))
    }
}

#[cfg(feature = "pui-core")]
impl<T> Seal for Range<Id<T>> {}
#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> PuiVecIndex<I> for Range<Id<I::Token>> {
    type SliceIndex = Range<usize>;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.start.token) && vec.ident.owns_token(&self.end.token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(&self) -> Self::SliceIndex { self.start.index..self.end.index }
}

#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> BuildPuiVecIndex<I> for Range<Id<I::Token>> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, ident: &I) -> Self {
        Id {
            index: slice_index.start,
            token: ident.token(),
        }..Id {
            index: slice_index.end,
            token: ident.token(),
        }
    }
}

#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVecAccess<T, I> for Range<Id<I::Token>> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.get_unchecked(PuiVecIndex::<I>::slice_index(self))
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.get_unchecked_mut(PuiVecIndex::<I>::slice_index(self))
    }
}

#[cfg(feature = "pui-core")]
impl<T> Seal for RangeInclusive<Id<T>> {}
#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> PuiVecIndex<I> for RangeInclusive<Id<I::Token>> {
    type SliceIndex = RangeInclusive<usize>;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool {
        if vec.ident.owns_token(&self.start().token) && vec.ident.owns_token(&self.end().token) {
            true
        } else {
            not_owned()
        }
    }

    fn slice_index(&self) -> Self::SliceIndex { self.start().index..=self.end().index }
}

#[cfg(feature = "pui-core")]
impl<I: OneShotIdentifier> BuildPuiVecIndex<I> for RangeInclusive<Id<I::Token>> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, ident: &I) -> Self {
        Id {
            index: *slice_index.start(),
            token: ident.token(),
        }..=Id {
            index: *slice_index.end(),
            token: ident.token(),
        }
    }
}

#[cfg(feature = "pui-core")]
impl<T, I: OneShotIdentifier> PuiVecAccess<T, I> for RangeInclusive<Id<I::Token>> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.get_unchecked(PuiVecIndex::<I>::slice_index(self))
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.get_unchecked_mut(PuiVecIndex::<I>::slice_index(self))
    }
}

impl Seal for usize {}
impl<I> PuiVecIndex<I> for usize {
    type SliceIndex = Self;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(&self) -> Self::SliceIndex { self.clone() }
}

impl<I> BuildPuiVecIndex<I> for usize {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, _: &I) -> Self { slice_index }
}

impl<T, I> PuiVecAccess<T, I> for usize {
    type Output = T;

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.vec.get_unchecked(self.clone())
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.vec.get_unchecked_mut(self.clone())
    }
}

impl Seal for RangeFull {}
impl<I> PuiVecIndex<I> for RangeFull {
    type SliceIndex = Self;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(&self) -> Self::SliceIndex { self.clone() }
}

impl<I> BuildPuiVecIndex<I> for RangeFull {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, _: &I) -> Self { slice_index }
}

impl<T, I> PuiVecAccess<T, I> for RangeFull {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.vec.get_unchecked(self.clone())
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.vec.get_unchecked_mut(self.clone())
    }
}

impl Seal for RangeTo<usize> {}
impl<I> PuiVecIndex<I> for RangeTo<usize> {
    type SliceIndex = Self;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(&self) -> Self::SliceIndex { self.clone() }
}

impl<I> BuildPuiVecIndex<I> for RangeTo<usize> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, _: &I) -> Self { slice_index }
}

impl<T, I> PuiVecAccess<T, I> for RangeTo<usize> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.vec.get_unchecked(self.clone())
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.vec.get_unchecked_mut(self.clone())
    }
}

impl Seal for RangeFrom<usize> {}
impl<I> PuiVecIndex<I> for RangeFrom<usize> {
    type SliceIndex = Self;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(&self) -> Self::SliceIndex { self.clone() }
}

impl<I> BuildPuiVecIndex<I> for RangeFrom<usize> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, _: &I) -> Self { slice_index }
}

impl<T, I> PuiVecAccess<T, I> for RangeFrom<usize> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.vec.get_unchecked(self.clone())
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.vec.get_unchecked_mut(self.clone())
    }
}

impl Seal for RangeToInclusive<usize> {}
impl<I> PuiVecIndex<I> for RangeToInclusive<usize> {
    type SliceIndex = Self;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(&self) -> Self::SliceIndex { self.clone() }
}

impl<I> BuildPuiVecIndex<I> for RangeToInclusive<usize> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, _: &I) -> Self { slice_index }
}

impl<T, I> PuiVecAccess<T, I> for RangeToInclusive<usize> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.vec.get_unchecked(self.clone())
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.vec.get_unchecked_mut(self.clone())
    }
}

impl Seal for Range<usize> {}
impl<I> PuiVecIndex<I> for Range<usize> {
    type SliceIndex = Self;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(&self) -> Self::SliceIndex { self.clone() }
}

impl<I> BuildPuiVecIndex<I> for Range<usize> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, _: &I) -> Self { slice_index }
}

impl<T, I> PuiVecAccess<T, I> for Range<usize> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.vec.get_unchecked(self.clone())
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.vec.get_unchecked_mut(self.clone())
    }
}

impl Seal for RangeInclusive<usize> {}
impl<I> PuiVecIndex<I> for RangeInclusive<usize> {
    type SliceIndex = Self;

    fn contained_in<T>(&self, vec: &PuiVec<T, I>) -> bool { vec.vec.get(self.clone()).is_some() }

    fn slice_index(&self) -> Self::SliceIndex { self.clone() }
}

impl<I> BuildPuiVecIndex<I> for RangeInclusive<usize> {
    unsafe fn new_unchecked(slice_index: Self::SliceIndex, _: &I) -> Self { slice_index }
}

impl<T, I> PuiVecAccess<T, I> for RangeInclusive<usize> {
    type Output = [T];

    unsafe fn get_unchecked<'a>(&self, vec: &'a PuiVec<T, I>) -> &'a Self::Output {
        vec.vec.get_unchecked(self.clone())
    }

    unsafe fn get_unchecked_mut<'a>(&self, vec: &'a mut PuiVec<T, I>) -> &'a mut Self::Output {
        vec.vec.get_unchecked_mut(self.clone())
    }
}
