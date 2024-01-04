use core::future::Future;
use futures::channel::oneshot::{self, Sender};
use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void}, mem::ManuallyDrop,
};

extern crate link_cplusplus;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub struct CefApp;

impl CefApp {
    pub fn new() -> Result<impl Future<Output = Self>, i32> {
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
            try_start_subprocess(argc, argv, Some(app_ready), sender_ptr)
        };

        if result != 0 {
            return Err(result);
        }

        Ok(async move {
            let _ = receiver.await;
            drop(sender);

            CefApp
        })
    }
}

fn get_posix_args() -> (c_int, *mut *mut c_char) {
    // create a vector of zero terminated strings
    let args = argv::iter()
        .map(|arg| arg as *const _)
        .collect::<Box<[_]>>();

    let argc = args.len() as c_int;
    let argv = Box::into_raw(args) as *mut *mut c_char;

    (argc, argv)
}
