use pui_core::Identifier;
use typsy::hlist::{Cons, Nil};

use seal::Seal;

use crate::{IdCell, IdentifierExt};
#[forbid(missing_docs)]
mod seal {
    pub trait Seal {
        fn __internal_find(&self, ptr: *mut ()) -> bool;
    }
}

pub trait GetAllMut<I>: Seal {
    type Output;

    fn get_all_mut(self, ident: I) -> Option<Self::Output>;
}

impl Seal for Nil {
    #[inline]
    fn __internal_find(&self, _: *mut ()) -> bool { false }
}

impl<T> GetAllMut<T> for Nil {
    type Output = Nil;

    fn get_all_mut(self, _: T) -> Option<Self::Output> { Some(Self) }
}

impl<T: ?Sized, R: Seal> Seal for Cons<&T, R> {
    fn __internal_find(&self, ptr: *mut ()) -> bool {
        let value = self.value as *const T as *const ();
        value == ptr || self.rest.__internal_find(ptr)
    }
}

impl<'a, T: ?Sized, R, I: ?Sized + Identifier> GetAllMut<&'a mut I> for Cons<&'a IdCell<T, I::Token>, R>
where
    R: GetAllMut<&'a mut I>,
{
    type Output = Cons<&'a mut T, R::Output>;

    fn get_all_mut(self, ident: &'a mut I) -> Option<Self::Output> {
        assert!(ident.owns(self.value));

        let ptr = self.value.as_ptr();

        if self.rest.__internal_find(ptr as *mut ()) {
            return None
        }

        Some(Cons {
            value: unsafe { &mut *ptr },
            rest: self.rest.get_all_mut(ident)?,
        })
    }
}
