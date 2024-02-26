use std::ffi::CString;

use cef_wrapper::cef_wrapper_sys::cef_load_library;


#[cfg(not(target_os = "macos"))]
compile_error!("This program can only be built for macOS.");

fn main() -> Result<(), i32> {
    let exec_dir = std::env::current_exe().unwrap();
    let parent_dir = exec_dir.parent().unwrap();
    let framework_path = "Chromium Embedded Framework.framework/Chromium Embedded Framework";
    let from_helper = "../../..";

    let path = parent_dir.join(from_helper).join(framework_path);

    let arg = CString::new(path.to_str().unwrap()).unwrap();
    let result = unsafe { cef_load_library(arg.as_ptr()) };

    match result {
        0 => Ok(()),
        e => {
            println!("Failed to load the CEF framework");
            Err(e)
        }
    }
}
