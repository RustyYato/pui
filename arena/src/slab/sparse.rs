use core::ops::{Index, IndexMut};

use crate::{
    base::sparse::{self as imp, Arena},
    version::Unversioned,
};

type Values<'a, T> = imp::Values<'a, T, Unversioned>;
type ValuesMut<'a, T> = imp::ValuesMut<'a, T, Unversioned>;
type IntoValues<T> = imp::IntoValues<T, Unversioned>;

imp_slab! {
    new const: Arena::INIT,
    slots: slots
}
