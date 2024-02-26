use cef_wrapper::cef_capi_sys::cef_execute_process;

use crate::structs::main_args::MainArgs;



pub fn execute_process(args: MainArgs) -> Result<(), i32> {
    let args = args.into();
    let result = unsafe { cef_execute_process(
        &args as *const _,
        std::ptr::null_mut(),
        std::ptr::null_mut()
    ) };

    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}
