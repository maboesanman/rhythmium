use cef_sys::{cef_life_span_handler_t, cef_browser_t};

use crate::base::initialize_cef_base_refcounted;


unsafe extern "C" fn on_before_close(
    this: *mut cef_life_span_handler_t,
    browser: *mut cef_browser_t
) {
    println!("on_before_close");
}

pub fn initialize_cef_life_span_handler(life_span_handler: &mut cef_life_span_handler_t) {
    life_span_handler.base.size = std::mem::size_of::<cef_life_span_handler_t>();
    initialize_cef_base_refcounted(life_span_handler as *mut _ as *mut _);

    life_span_handler.on_before_close = Some(on_before_close);
}