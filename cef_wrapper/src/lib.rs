use core::future::Future;
use std::{os::raw::{c_void, c_char, c_int}, ffi::CString};
use futures::channel::oneshot::{self, Sender};

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
            let (argc, argv) = get_c_args();
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

fn get_c_args() -> (c_int, *mut *mut c_char) {
    // create a vector of zero terminated strings
    let mut args = std::env::args().map(|arg| CString::new(arg).unwrap() ).collect::<Vec<CString>>();
    // convert the strings to raw pointers
    let c_args = args.iter_mut().map(|arg| arg.as_ptr().cast_mut()).collect::<Vec<*mut c_char>>();
    
    let argc = c_args.len() as c_int;
    let argv = c_args.as_ptr().cast_mut();

    (argc, argv)
}
