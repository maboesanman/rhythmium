#![feature(hash_raw_entry)]
#![feature(strict_provenance)]
#![feature(never_type)]
#![feature(fn_traits)]
#![feature(type_alias_impl_trait)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(async_fn_in_trait)]

pub mod source;
pub mod transposer;
mod util;

// #[cfg(test)]
// mod testing;
