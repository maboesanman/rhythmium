use cef_sys::{cef_app_t, _cef_string_utf16_t};

use crate::base::initialize_cef_base_refcounted;

unsafe extern "C" fn on_before_command_line_processing(
    _self: *mut cef_app_t,
    _process_type: *const _cef_string_utf16_t,
    _command_line: *mut cef_sys::cef_command_line_t,
) {
    println!("on_before_command_line_processing");
}

unsafe extern "C" fn on_register_custom_schemes(
    _self: *mut cef_app_t,
    _registrar: *mut cef_sys::cef_scheme_registrar_t,
) {
    println!("on_register_custom_schemes");
}

unsafe extern "C" fn get_resource_bundle_handler(
    _self: *mut cef_app_t,
) -> *mut cef_sys::cef_resource_bundle_handler_t {
    println!("get_resource_bundle_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_browser_process_handler(
    _self: *mut cef_app_t,
) -> *mut cef_sys::cef_browser_process_handler_t {
    println!("get_browser_process_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_render_process_handler(
    _self: *mut cef_app_t,
) -> *mut cef_sys::cef_render_process_handler_t {
    println!("get_render_process_handler");

    std::ptr::null_mut()
}

pub fn initialize_cef_app(app: &mut cef_app_t) {
    app.base.size = std::mem::size_of::<cef_app_t>();
    initialize_cef_base_refcounted(app as *mut _ as *mut _);

    app.on_before_command_line_processing = Some(on_before_command_line_processing);
    app.on_register_custom_schemes = Some(on_register_custom_schemes);
    app.get_resource_bundle_handler = Some(get_resource_bundle_handler);
    app.get_browser_process_handler = Some(get_browser_process_handler);
    app.get_render_process_handler = Some(get_render_process_handler);
}
