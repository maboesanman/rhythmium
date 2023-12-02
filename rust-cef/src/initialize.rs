use cef_sys::{cef_initialize, cef_main_args_t};

use crate::{app::App, settings::Settings, util::cef_arc::CefArc};

pub fn initialize(args: Vec<String>, settings: &Settings, app: Option<CefArc<App>>) -> bool {
    let argc = args.len() as std::ffi::c_int;
    let mut args_pointers = args
        .into_iter()
        .map(std::ffi::CString::new)
        .filter_map(Result::ok)
        .map(|arg| arg.as_ptr().cast_mut())
        .chain([std::ptr::null_mut()])
        .collect::<Vec<_>>();

    let argv = args_pointers.as_mut_ptr();

    let c_args = cef_main_args_t { argc, argv };

    let settings = settings.get_cef_settings();

    let app = match app {
        Some(app) => app.into_raw().cast(),
        None => std::ptr::null_mut(),
    };

    unsafe { cef_initialize(&c_args, &settings, app, std::ptr::null_mut()) == 1 }
}
