use core::ops::{Index, IndexMut};

use crate::{
    base::hop::{self as imp, Arena, Key},
    version::DefaultVersion,
};

type Values<'a, T> = imp::Values<'a, T, DefaultVersion>;
type ValuesMut<'a, T> = imp::ValuesMut<'a, T, DefaultVersion>;
type IntoValues<T> = imp::IntoValues<T, DefaultVersion>;

imp_slot_map! {
    new: Arena::with_ident(()),
    slots: len
}
