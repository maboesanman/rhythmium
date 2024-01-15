#![feature(cfg_sanitize)]
#![feature(arbitrary_self_types)]
#![feature(ptr_metadata)]

#[macro_use]
pub mod macros;

pub mod c_to_rust;
pub mod enums;
pub mod rust_to_c;
pub mod structs;
pub mod util;
