use cef_sys::cef_client_t;

use crate::base::initialize_cef_base_refcounted;

// // ----------------------------------------------------------------------------
// // struct cef_client_t
// // ----------------------------------------------------------------------------

// ///
// // Implement this structure to provide handler implementations.
// ///

// ///
// // Return the handler for context menus. If no handler is
// // provided the default implementation will be used.
// ///

// struct _cef_context_menu_handler_t* CEF_CALLBACK get_context_menu_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_context_menu_handler\n");
// return NULL;
// }

// ///
// // Return the handler for dialogs. If no handler is provided the default
// // implementation will be used.
// ///
// struct _cef_dialog_handler_t* CEF_CALLBACK get_dialog_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_dialog_handler\n");
// return NULL;
// }

// ///
// // Return the handler for browser display state events.
// ///
// struct _cef_display_handler_t* CEF_CALLBACK get_display_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_display_handler\n");
// return NULL;
// }

// ///
// // Return the handler for download events. If no handler is returned downloads
// // will not be allowed.
// ///
// struct _cef_download_handler_t* CEF_CALLBACK get_download_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_download_handler\n");
// return NULL;
// }

// ///
// // Return the handler for drag events.
// ///
// struct _cef_drag_handler_t* CEF_CALLBACK get_drag_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_drag_handler\n");
// return NULL;
// }

// ///
// // Return the handler for focus events.
// ///
// struct _cef_focus_handler_t* CEF_CALLBACK get_focus_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_focus_handler\n");
// return NULL;
// }

// ///
// // Return the handler for geolocation permissions requests. If no handler is
// // provided geolocation access will be denied by default.
// ///
// struct _cef_geolocation_handler_t* CEF_CALLBACK get_geolocation_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_geolocation_handler\n");
// return NULL;
// }

// ///
// // Return the handler for JavaScript dialogs. If no handler is provided the
// // default implementation will be used.
// ///
// struct _cef_jsdialog_handler_t* CEF_CALLBACK get_jsdialog_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_jsdialog_handler\n");
// return NULL;
// }

// ///
// // Return the handler for keyboard events.
// ///
// struct _cef_keyboard_handler_t* CEF_CALLBACK get_keyboard_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_keyboard_handler\n");
// return NULL;
// }

// ///
// // Return the handler for browser life span events.
// ///
// struct _cef_life_span_handler_t* CEF_CALLBACK get_life_span_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_life_span_handler\n");
// // Implemented!
// return &g_life_span_handler;
// }

// ///
// // Return the handler for browser load status events.
// ///
// struct _cef_load_handler_t* CEF_CALLBACK get_load_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_load_handler\n");
// return NULL;
// }

// ///
// // Return the handler for off-screen rendering events.
// ///
// struct _cef_render_handler_t* CEF_CALLBACK get_render_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_render_handler\n");
// return NULL;
// }

// ///
// // Return the handler for browser request events.
// ///
// struct _cef_request_handler_t* CEF_CALLBACK get_request_handler(
//     struct _cef_client_t* self) {
// DEBUG_CALLBACK("get_request_handler\n");
// return NULL;
// }

// ///
// // Called when a new message is received from a different process. Return true
// // (1) if the message was handled or false (0) otherwise. Do not keep a
// // reference to or attempt to access the message outside of this callback.
// ///
// int CEF_CALLBACK on_process_message_received(
//     struct _cef_client_t* self,
//     struct _cef_browser_t* browser, cef_process_id_t source_process,
//     struct _cef_process_message_t* message) {
// DEBUG_CALLBACK("on_process_message_received\n");
// return 0;
// }

// now in rust:
unsafe extern "C" fn get_context_menu_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_context_menu_handler_t {
    println!("get_context_menu_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_dialog_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_dialog_handler_t {
    println!("get_dialog_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_display_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_display_handler_t {
    println!("get_display_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_download_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_download_handler_t {
    println!("get_download_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_drag_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_drag_handler_t {
    println!("get_drag_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_focus_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_focus_handler_t {
    println!("get_focus_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_jsdialog_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_jsdialog_handler_t {
    println!("get_jsdialog_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_keyboard_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_keyboard_handler_t {
    println!("get_keyboard_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_life_span_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_life_span_handler_t {
    println!("get_life_span_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_load_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_load_handler_t {
    println!("get_load_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_render_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_render_handler_t {
    println!("get_render_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn get_request_handler(
    _self: *mut cef_client_t,
) -> *mut cef_sys::cef_request_handler_t {
    println!("get_request_handler");

    std::ptr::null_mut()
}

unsafe extern "C" fn on_process_message_received(
    _self: *mut cef_client_t,
    _browser: *mut cef_sys::cef_browser_t,
    _frame: *mut cef_sys::cef_frame_t,
    _source_process: cef_sys::cef_process_id_t,
    _message: *mut cef_sys::cef_process_message_t,
) -> std::os::raw::c_int {
    println!("on_process_message_received");

    0
}


pub fn initialize_cef_client(client: &mut cef_client_t) {
    client.base.size = std::mem::size_of::<cef_client_t>();
    initialize_cef_base_refcounted(client as *mut _ as *mut _);

    // callbacks
    client.get_context_menu_handler = Some(get_context_menu_handler);
    client.get_dialog_handler = Some(get_dialog_handler);
    client.get_display_handler = Some(get_display_handler);
    client.get_download_handler = Some(get_download_handler);
    client.get_drag_handler = Some(get_drag_handler);
    client.get_focus_handler = Some(get_focus_handler);
    client.get_jsdialog_handler = Some(get_jsdialog_handler);
    client.get_keyboard_handler = Some(get_keyboard_handler);
    client.get_life_span_handler = Some(get_life_span_handler);
    client.get_load_handler = Some(get_load_handler);
    client.get_render_handler = Some(get_render_handler);
    client.get_request_handler = Some(get_request_handler);
    client.on_process_message_received = Some(on_process_message_received);
}