#![no_std]

extern crate alloc as std;

pub mod version;

pub mod base {
    pub mod dense;
    pub mod hop;
    pub mod sparse;
}

#[cfg(feature = "slab")]
pub mod slab;
#[cfg(feature = "slot_map")]
pub mod slot_map;
