use core::ops::{Index, IndexMut};

use crate::{
    hop::{self as imp, Arena},
    version::Unversioned,
};

type Values<'a, T> = imp::Values<'a, T, Unversioned>;
type ValuesMut<'a, T> = imp::ValuesMut<'a, T, Unversioned>;
type IntoValues<T> = imp::IntoValues<T, Unversioned>;

imp_slab! {
    new: Arena::with_ident(()),
    slots: len
}
