use core::ops::{Index, IndexMut};

use crate::{
    base::{
        dense::{self as imp, Arena},
        sparse as key,
    },
    version::DefaultVersion,
};

pub type Values<'a, T> = core::slice::Iter<'a, T>;
pub type ValuesMut<'a, T> = core::slice::IterMut<'a, T>;
pub type IntoValues<T> = std::vec::IntoIter<T>;

imp_slot_map! {
    new: Arena::with_ident(()),
    slots: len
}
