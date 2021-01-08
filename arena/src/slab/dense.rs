use core::ops::{Index, IndexMut};

use crate::{
    base::dense::{self as imp, Arena},
    version::Unversioned,
};

pub type Iter<'a, T> = core::slice::Iter<'a, T>;
pub type IterMut<'a, T> = core::slice::IterMut<'a, T>;
pub type IntoIter<T> = std::vec::IntoIter<T>;

pub type Drain<'a, T> = imp::Drain<'a, T, (), Unversioned>;
pub type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, (), Unversioned, F>;

imp_slab! {
    new: Arena::with_ident(()),
    slots: len
}
