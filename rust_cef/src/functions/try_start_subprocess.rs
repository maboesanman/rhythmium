use std::ffi::CString;

#[cfg(target_os = "macos")]
use cef_wrapper::cef_wrapper_sys::cef_load_library;

use crate::structs::main_args::MainArgs;

#[cfg(target_os = "macos")]
pub fn try_start_subprocess(_main_args: &MainArgs) {
    #[cfg(feature = "bundled")]
    try_start_subprocess_from_rel_cef_framework_path("../Frameworks");
    #[cfg(not(feature = "bundled"))]
    try_start_subprocess_from_rel_cef_framework_path("../../build/lib/Frameworks");
}

#[cfg(target_os = "macos")]
pub fn try_start_subprocess_from_rel_cef_framework_path(rel_cef_framework_path: &str) {
    let exec_dir = std::env::current_exe().unwrap();
    let parent_dir = exec_dir.parent().unwrap();
    let rel_chromium_framework_path =
        "Chromium Embedded Framework.framework/Chromium Embedded Framework";

    let path = parent_dir
        .join(rel_cef_framework_path)
        .join(rel_chromium_framework_path);

    let arg = CString::new(path.to_str().unwrap()).unwrap();
    let result = unsafe { cef_load_library(arg.as_ptr()) };

    if result == 0 {
        panic!("Failed to load the CEF framework");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn try_start_subprocess(main_args: &MainArgs) {
    use crate::{c_to_rust::command_line, functions::cef_execute_process::execute_process};

    let command_line = command_line::CommandLine::new_from_main_args(main_args.clone());

    match command_line.get_process_type() {
        command_line::ProcessType::Browser => {
            return;
        }
        command_line::ProcessType::Render => {}
        command_line::ProcessType::Other => {}
    }

    match execute_process(main_args.clone()) {
        Ok(_) => {}
        Err(e) => {
            std::process::exit(e);
        }
    }
}
