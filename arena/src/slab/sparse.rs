use core::ops::{Index, IndexMut};

use crate::{
    base::sparse::{self as imp, Arena},
    version::Unversioned,
};

pub type Iter<'a, T> = imp::Iter<'a, T, Unversioned>;
pub type IterMut<'a, T> = imp::IterMut<'a, T, Unversioned>;
pub type IntoIter<T> = imp::IntoIter<T, Unversioned>;

imp_slab! {
    new const: Arena::INIT,
    slots: slots
}
