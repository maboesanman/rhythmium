use cef_sys::{cef_execute_process, cef_main_args_t};

use crate::{app::App, util::cef_arc::CefArc};

pub fn execute_process(args: Vec<String>, app: Option<CefArc<App>>) -> Result<i32, ()> {
    let argc = args.len() as std::ffi::c_int;
    let mut args_pointers = args
        .into_iter()
        .map(std::ffi::CString::new)
        .filter_map(Result::ok)
        .map(|arg| arg.as_ptr() as *mut std::ffi::c_char)
        .chain([std::ptr::null_mut()])
        .collect::<Vec<_>>();

    let argv = args_pointers.as_mut_ptr();

    let c_args = cef_main_args_t { argc, argv };

    let app = match app {
        Some(app) => app.into_mut_ptr().cast(),
        None => std::ptr::null_mut(),
    };

    let result = unsafe { cef_execute_process(&c_args, app, std::ptr::null_mut()) };

    if result >= 0 {
        Ok(result)
    } else {
        Err(())
    }
}
