use core::ops::{Index, IndexMut};

use crate::{
    base::hop::{self as imp, self as key, Arena},
    version::DefaultVersion,
};

pub type Values<'a, T> = imp::Values<'a, T, DefaultVersion>;
pub type ValuesMut<'a, T> = imp::ValuesMut<'a, T, DefaultVersion>;
pub type IntoValues<T> = imp::IntoValues<T, DefaultVersion>;

imp_slot_map! {
    new: Arena::with_ident(()),
    slots: len
}
