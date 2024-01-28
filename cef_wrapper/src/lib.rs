use core::future::Future;
use futures::channel::oneshot::{self, Sender};
use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void},
};

extern crate link_cplusplus;

pub mod cef_capi_sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings_c.rs"));
}

mod cef_wrapper_sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings_cpp.rs"));
}

pub use cef_capi_sys::cef_rect_t as CefRect;

pub fn init() -> Result<impl Future<Output = ()>, i32> {
    let (sender, receiver) = oneshot::channel::<()>();

    let mut sender = Box::new(Some(sender));

    extern "C" fn app_ready(sender: *mut c_void) {
        let sender = sender.cast::<Option<Sender<()>>>();
        let sender = unsafe { sender.as_mut().unwrap() };
        let sender = sender.take().expect("app_ready called twice");
        sender.send(()).expect("app initialization failed");
    }

    let sender_ptr = sender.as_mut() as *mut _ as *mut c_void;
    let result = unsafe {
        let (argc, argv) = get_posix_args();
        cef_wrapper_sys::try_start_subprocess(argc, argv, Some(app_ready), sender_ptr)
    };

    if result != 0 {
        return Err(result);
    }

    Ok(async move {
        let _ = receiver.await;
        drop(sender);
    })
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

pub fn do_cef_message_loop_work() {
    unsafe { cef_capi_sys::cef_do_message_loop_work() }
}
