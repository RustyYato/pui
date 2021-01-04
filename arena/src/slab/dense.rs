use core::ops::{Index, IndexMut};

use crate::{
    dense::{self as imp, Arena},
    version::Unversioned,
};

type Values<'a, T> = core::slice::Iter<'a, T>;
type ValuesMut<'a, T> = core::slice::IterMut<'a, T>;
type IntoValues<T> = std::vec::IntoIter<T>;

imp_slab! {
    new: Arena::with_ident(()),
    slots: len
}
