use core::cell::Cell;

pub use Default;

pub type LocalFlag<T> = Cell<Option<crate::scalar::OpaqueScalar<T>>>;

pub use core::compile_error;
#[cfg(feature = "std")]
pub use std::thread_local;

#[cfg(not(feature = "std"))]
pub use crate::thread_local;

#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "std"))]
macro_rules! thread_local {
    ($($input:tt)*) => {
        $crate::export::compile_error! { "`thread_local` can only be used if the `std` feature is turned on in `pui`" }
    };
}

pub struct NoSendSync(&'static Cell<()>);
