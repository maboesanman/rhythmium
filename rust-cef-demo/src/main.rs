#![feature(ptr_metadata)]
use std::{env, mem::size_of};

use cef_sys::{cef_main_args_t, cef_initialize, cef_base_ref_counted_t, cef_execute_process, _cef_settings_t};

use crate::{app::initialize_cef_app, strings::into_cef_str};

mod base;
mod app;
mod life_span_handler;
mod client;
mod strings;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    println!("args: {:?}", args);

    let argc = args.len() as std::ffi::c_int;
    let mut args_pointers = args.into_iter()
        .map(std::ffi::CString::new)
        .filter_map(Result::ok)
        .map(|arg| arg.as_ptr() as *mut std::ffi::c_char)
        .chain([std::ptr::null_mut()])
        .collect::<Vec<_>>();
    let argv = args_pointers.as_mut_ptr();

    let main_args = cef_main_args_t {
        argc,
        argv,
    };

    let mut app = cef_sys::cef_app_t {
        base: cef_base_ref_counted_t {
            size: 0,
            add_ref: None,
            release: None,
            has_one_ref: None,
            has_at_least_one_ref: None,
        },
        on_before_command_line_processing: None,
        on_register_custom_schemes: None,
        get_resource_bundle_handler: None,
        get_browser_process_handler: None,
        get_render_process_handler: None,
    };

    initialize_cef_app(&mut app);

    let code = unsafe { cef_execute_process(&main_args, &mut app as *mut _, std::ptr::null_mut()) };

    if code >= 0 {
        std::process::exit(code);
    }

    let settings = _cef_settings_t {
        size: size_of::<_cef_settings_t>(),
        no_sandbox: 0,
        browser_subprocess_path: into_cef_str(""),
        framework_dir_path: into_cef_str(""),
        main_bundle_path: into_cef_str(""),
        chrome_runtime: 0,
        multi_threaded_message_loop: 0,
        external_message_pump: 0,
        windowless_rendering_enabled: 0,
        command_line_args_disabled: 0,
        cache_path: into_cef_str(""),
        root_cache_path: into_cef_str(""),
        persist_session_cookies: 0,
        persist_user_preferences: 0,
        user_agent: into_cef_str(""),
        user_agent_product: into_cef_str(""),
        locale: into_cef_str(""),
        log_file: into_cef_str(""),
        log_severity: 0,
        javascript_flags: into_cef_str(""),
        resources_dir_path: into_cef_str(""),
        locales_dir_path: into_cef_str(""),
        pack_loading_disabled: 0,
        remote_debugging_port: 0,
        uncaught_exception_stack_size: 0,
        background_color: 0,
        accept_language_list: into_cef_str(""),
        cookieable_schemes_list: into_cef_str(""),
        cookieable_schemes_exclude_defaults: 0,
    };

    // let code = unsafe { cef_initialize(&main_args, &settings, &mut app, std::ptr::null_mut()) };
}
