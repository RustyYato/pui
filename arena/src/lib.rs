#![no_std]

extern crate alloc as std;

pub mod version;

pub mod base {
    pub mod dense;
    pub mod hop;
    pub mod sparse;
}

pub mod slab;
pub mod slot_map;
