use std::ffi::{c_int, c_void};
extern "C" {
    fn main_ffi(argc: c_int, argv: *const c_void) -> c_int;
}

pub fn call_the_library() -> i32 {
    unsafe { main_ffi(0, std::ptr::null()) }
}
