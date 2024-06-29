use rust_cef::{
    functions::{
        cef_execute_process::execute_process,
        try_start_subprocess::try_start_subprocess_from_rel_cef_framework_path,
    },
    structs::main_args::MainArgs,
};

fn main() -> Result<(), i32> {
    #[cfg(target_os = "macos")]
    return inner();

    #[cfg(not(target_os = "macos"))]
    panic!("This helper program can only be built for macOS.");
}

#[cfg(target_os = "macos")]
fn inner() -> Result<(), i32> {
    try_start_subprocess_from_rel_cef_framework_path("../../..");
    execute_process(MainArgs::from_env())
}
