use std::ffi::{CStr, CString};

#[cfg(target_os = "macos")]
use cef_wrapper::cef_wrapper_sys::cef_load_library;

use crate::structs::main_args::MainArgs;

#[cfg(target_os = "macos")]
pub fn try_start_subprocess(main_args: &MainArgs) {
    let exec_dir = std::env::current_exe().unwrap();
    let parent_dir = exec_dir.parent().unwrap();
    let framework_path = "Chromium Embedded Framework.framework/Chromium Embedded Framework";
    let from_main = "../Frameworks";
    let from_helper = "../../../";
    let from_main = "/Users/mason/Source/github.com/maboesanman/rhythmium/build/lib/Frameworks";

    let path = parent_dir.join(from_main).join(framework_path);

    let arg = CString::new(path.to_str().unwrap()).unwrap();
    let result = unsafe { cef_load_library(arg.as_ptr()) };

    if result == 0 {
        panic!("Failed to load the CEF framework");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn try_start_subprocess(main_args: &MainArgs) {
    
}