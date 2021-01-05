use core::ops::{Index, IndexMut};

use crate::{
    base::hop::{self as imp, self as key, Arena},
    version::DefaultVersion,
};

pub type Iter<'a, T> = imp::Iter<'a, T, DefaultVersion>;
pub type IterMut<'a, T> = imp::IterMut<'a, T, DefaultVersion>;
pub type IntoIter<T> = imp::IntoIter<T, DefaultVersion>;

imp_slot_map! {
    new: Arena::with_ident(()),
    slots: len
}
