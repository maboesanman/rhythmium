#![feature(ptr_metadata)]

use cef_sys::{cef_main_args_t, cef_base_ref_counted_t, cef_execute_process, cef_command_line_create};

use crate::temp::{app::initialize_cef_app, strings::into_cef_str};

mod temp;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    println!("args: {:?}", args);

    let argc = args.len() as std::ffi::c_int;
    let mut args_pointers = args
        .into_iter()
        .map(std::ffi::CString::new)
        .filter_map(Result::ok)
        .map(|arg| arg.as_ptr() as *mut std::ffi::c_char)
        .chain([std::ptr::null_mut()])
        .collect::<Vec<_>>();
    let argv = args_pointers.as_mut_ptr();

    let main_args = cef_main_args_t { argc, argv };

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
    println!("initialize app");

    let command_line = unsafe { &mut *cef_command_line_create() };
    // let app = match get_process_type(command_line) {
    //     ProcessType::Browser => todo!(),
    //     ProcessType::Renderer => todo!(),
    //     ProcessType::Other => todo!(),
    // };

    // println!("execute process");
    // let code = unsafe { cef_execute_process(&main_args, &mut app as *mut _, std::ptr::null_mut()) };

    println!("exiting");
}

enum ProcessType {
    Browser,
    Renderer,
    Other,
}

fn get_process_type(command_line: &mut cef_sys::cef_command_line_t) -> ProcessType {
    if unsafe { command_line.has_switch.unwrap()(command_line, &into_cef_str("type")) == 0 } {
        return ProcessType::Browser;
    }

    if unsafe { command_line.get_switch_value.unwrap()(command_line, &into_cef_str("type")) == &mut into_cef_str("renderer") } {
        return ProcessType::Renderer;
    }

    ProcessType::Other
}