#![feature(type_alias_impl_trait)]
#![feature(ptr_metadata)]
#![feature(btree_set_entry)]
#![feature(assert_matches)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(async_fn_in_trait)]
#![recursion_limit = "1024"]

pub mod source;

/// The transposer module contains the types needed to create your own transposer,
/// as well as the step module for driving the transposer.
#[warn(missing_docs)]
pub mod transposer;
mod util;
