use core::ops::{Index, IndexMut};

use crate::{
    base::{
        dense::{self as imp, Arena},
        sparse as key,
    },
    version::DefaultVersion,
};

pub type Iter<'a, T> = core::slice::Iter<'a, T>;
pub type IterMut<'a, T> = core::slice::IterMut<'a, T>;
pub type IntoIter<T> = std::vec::IntoIter<T>;

imp_slot_map! {
    new: Arena::with_ident(()),
    slots: len
}
