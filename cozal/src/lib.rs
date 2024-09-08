#![feature(waker_getters)]
#![feature(hash_raw_entry)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(async_fn_in_trait)]
#![feature(strict_provenance)]

pub mod source;
pub mod transposer;
mod util;

// #[cfg(test)]
// mod testing;
