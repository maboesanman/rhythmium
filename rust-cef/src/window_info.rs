use std::os::raw::c_void;

use cef_sys::cef_window_info_t;

use crate::rect::Rect;


#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub window_name: String,
    pub bounds: Rect,
    pub hidden: bool,
    pub parent_view: *mut c_void,
    pub windowless_rendering_enabled: bool,

    #[cfg(target_os = "windows")]
    pub shared_texture_enabled: bool,

    pub external_begin_frame_enabled: bool,

    pub view: *mut c_void,
}

impl WindowInfo {
    pub fn get_cef_window_info(&self) -> cef_window_info_t {
        cef_window_info_t {
            window_name: crate::util::cef_string::str_into_cef_string_utf16(&self.window_name),
            bounds: self.bounds.get_cef_rect(),
            hidden: crate::util::wrap_boolean::wrap_boolean(self.hidden),
            parent_view: self.parent_view,
            windowless_rendering_enabled: crate::util::wrap_boolean::wrap_boolean(self.windowless_rendering_enabled),

            #[cfg(target_os = "windows")]
            shared_texture_enabled: crate::util::wrap_boolean::wrap_boolean(self.shared_texture_enabled),
            #[cfg(not(target_os = "windows"))]
            shared_texture_enabled: crate::util::wrap_boolean::wrap_boolean(false),

            external_begin_frame_enabled: crate::util::wrap_boolean::wrap_boolean(self.external_begin_frame_enabled),
            view: self.view,
        }
    }
}