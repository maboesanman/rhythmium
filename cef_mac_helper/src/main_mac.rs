use rust_cef::{
    functions::{
        cef_execute_process::execute_process,
        try_start_subprocess::try_start_subprocess_from_rel_cef_framework_path,
    },
    structs::main_args::MainArgs,
};

pub fn main() -> Result<(), i32> {
    try_start_subprocess_from_rel_cef_framework_path("../../..");

    execute_process(MainArgs::from_env())
}
