use cef_sys::{_cef_string_utf16_t, cef_app_t};

use super::{base::initialize_cef_base_refcounted, strings::into_cef_str};

unsafe extern "C" fn on_before_command_line_processing(
    _self: *mut cef_app_t,
    _process_type: *const _cef_string_utf16_t,
    command_line: *mut cef_sys::cef_command_line_t,
) {
    let command_line: &mut _ = &mut *command_line;

    // don't ask about keychain on macos.
    command_line.append_switch.unwrap()(command_line, &into_cef_str("use-mock-keychain"));
    // command_line.append_switch.unwrap()(command_line, &into_cef_str("disable-gpu"));
    // command_line.append_switch.unwrap()(command_line, &into_cef_str("disable-gpu-sandbox"));
    // command_line.append_switch.unwrap()(command_line, &into_cef_str("--disable-gpu"));
}

unsafe extern "C" fn on_register_custom_schemes(
    _self: *mut cef_app_t,
    _registrar: *mut cef_sys::cef_scheme_registrar_t,
) {
}

unsafe extern "C" fn get_resource_bundle_handler(
    _self: *mut cef_app_t,
) -> *mut cef_sys::cef_resource_bundle_handler_t {
    std::ptr::null_mut()
}

unsafe extern "C" fn get_browser_process_handler(
    _self: *mut cef_app_t,
) -> *mut cef_sys::cef_browser_process_handler_t {
    std::ptr::null_mut()
}

unsafe extern "C" fn get_render_process_handler(
    _self: *mut cef_app_t,
) -> *mut cef_sys::cef_render_process_handler_t {
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
