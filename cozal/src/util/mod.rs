#![feature(hash_raw_entry)]
#![feature(waker_getters)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(unused)]

pub mod dummy_waker;
pub mod extractable_rpds;
pub mod observing_waker;
pub mod option_min;
pub mod replace_mut;
pub mod replace_waker;
pub mod stack_waker;
pub mod vecdeque_helpers;
