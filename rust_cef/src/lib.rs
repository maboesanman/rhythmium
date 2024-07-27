#![feature(allocator_api)]
#![feature(arbitrary_self_types)]
#![feature(cfg_sanitize)]
#![feature(iter_collect_into)]
#![feature(ptr_metadata)]
#![allow(non_upper_case_globals)]
#![allow(clippy::new_ret_no_self)]

pub mod c_to_rust;
pub mod enums;
pub mod functions;
pub mod rust_to_c;
pub mod structs;
pub mod util;
