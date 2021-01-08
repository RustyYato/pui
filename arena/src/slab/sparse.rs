use core::ops::{Index, IndexMut};

use crate::{
    base::sparse::{self as imp, Arena},
    version::Unversioned,
};

pub type Iter<'a, T> = imp::Iter<'a, T, Unversioned>;
pub type IterMut<'a, T> = imp::IterMut<'a, T, Unversioned>;
pub type IntoIter<T> = imp::IntoIter<T, Unversioned>;

pub type Drain<'a, T> = imp::Drain<'a, T, Unversioned>;
pub type DrainFilter<'a, T, F> = imp::DrainFilter<'a, T, Unversioned, F>;

imp_slab! {
    new const: Arena::INIT,
    slots: slots
}
