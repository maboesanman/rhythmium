use std::ffi::CString;

use cef_wrapper::cef_wrapper_sys::cef_load_library;
use rust_cef::{functions::cef_execute_process::execute_process, structs::main_args::MainArgs};

pub fn main() -> Result<(), i32> {
    let exec_dir = std::env::current_exe().unwrap();
    let parent_dir = exec_dir.parent().unwrap();
    let framework_path = "Chromium Embedded Framework.framework/Chromium Embedded Framework";
    let from_helper = "../../..";

    let path = parent_dir.join(from_helper).join(framework_path);

    let arg = CString::new(path.to_str().unwrap()).unwrap();
    let result = unsafe { cef_load_library(arg.as_ptr()) };

    if result == 0 {
        panic!("Failed to load the CEF framework");
    }

    execute_process(MainArgs::from_env())
}
