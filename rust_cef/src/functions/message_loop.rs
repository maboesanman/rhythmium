use cef_wrapper::cef_capi_sys::cef_do_message_loop_work;

pub fn do_message_loop_work() {
    unsafe {
        cef_do_message_loop_work();
    }
}
