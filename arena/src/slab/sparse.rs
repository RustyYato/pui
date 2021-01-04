use core::ops::{Index, IndexMut};

use crate::{
    base::sparse::{self as imp, Arena},
    version::Unversioned,
};

pub type Values<'a, T> = imp::Values<'a, T, Unversioned>;
pub type ValuesMut<'a, T> = imp::ValuesMut<'a, T, Unversioned>;
pub type IntoValues<T> = imp::IntoValues<T, Unversioned>;

imp_slab! {
    new const: Arena::INIT,
    slots: slots
}
