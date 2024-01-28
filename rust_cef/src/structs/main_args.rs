use std::ffi::{CString, c_int, c_char};

use cef_wrapper::cef_capi_sys::cef_main_args_t;


pub struct MainArgs {
    pub args: Vec<String>,
}

impl MainArgs {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
        }
    }

    pub fn from_env() -> Self {
        Self {
            args: std::env::args().collect(),
        }
    }
}

impl From<cef_main_args_t> for MainArgs {
    fn from(main_args: cef_main_args_t) -> Self {
        unimplemented!()
    }
}

impl From<MainArgs> for cef_main_args_t {
    fn from(val: MainArgs) -> Self {
        let args: Box<[_]> = val.args
            .into_iter()
            .filter_map(|s| match CString::new(s) {
                Ok(s) => Some(s.into_raw()),
                Err(_) => None,
            })
            .collect();

        let argc = args.len() as c_int;
        let argv = Box::into_raw(args) as *mut *mut c_char;

        cef_main_args_t {
            argc,
            argv,
        }
    }
}
