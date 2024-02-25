use std::ptr;

use cef_wrapper::cef_capi_sys::cef_window_info_t;

use crate::util::{cef_string, wrap_boolean::wrap_boolean};

use super::geometry::Rect;

pub struct WindowInfo {
    pub window_name: String,
    pub bounds: Rect,
    pub hidden: bool,
    pub windowless_rendering_enabled: bool,
    pub external_begin_frame_enabled: bool,
    // TODO:
    // parent_view (platform specific)
    // view (platform specific)
    // shared_texture_enabled (windows only)
}

impl From<&WindowInfo> for cef_window_info_t {
    fn from(val: &WindowInfo) -> Self {
        cef_window_info_t {
            window_name: cef_string::str_into_cef_string_utf16(&val.window_name),
            bounds: val.bounds.into(),
            hidden: wrap_boolean(val.hidden),
            parent_view: ptr::null_mut(),
            windowless_rendering_enabled: wrap_boolean(val.windowless_rendering_enabled),
            shared_texture_enabled: wrap_boolean(false),
            external_begin_frame_enabled: wrap_boolean(val.external_begin_frame_enabled),
            view: ptr::null_mut(),
        }
    }
}
