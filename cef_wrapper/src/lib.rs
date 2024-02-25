use std::{
    ffi::CString,
    os::raw::{c_char, c_int},
};

extern crate link_cplusplus;

pub mod cef_capi_sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(non_upper_case_globals)]
    #![allow(clippy::type_complexity)]
    
    include!(concat!(env!("OUT_DIR"), "/bindings_c.rs"));
}

mod cef_wrapper_sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(non_upper_case_globals)]
    #![allow(clippy::type_complexity)]

    include!(concat!(env!("OUT_DIR"), "/bindings_cpp.rs"));
}

pub use cef_capi_sys::cef_rect_t as CefRect;

pub fn init() -> Result<(), i32> {
    let (argc, argv) = get_posix_args();
    let result = unsafe { cef_wrapper_sys::try_start_subprocess(argc, argv) };
    match result {
        0 => Ok(()),
        e => Err(e),
    }
}

fn get_posix_args() -> (c_int, *mut *mut c_char) {
    // create a vector of zero terminated strings
    let args: Box<[_]> = std::env::args()
        .filter_map(|s| match CString::new(s) {
            Ok(s) => Some(s.into_raw()),
            Err(_) => None,
        })
        .collect();

    let argc = args.len() as c_int;
    let argv = Box::into_raw(args) as *mut *mut c_char;

    (argc, argv)
}
