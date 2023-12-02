use std::os::raw::c_void;

use cef_sys::cef_window_info_t;

use crate::{rect::Rect, util::cef_string::str_into_cef_string_utf16};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub window_name: String,
    pub bounds: Rect,
    pub hidden: bool,
    pub parent_view: *mut c_void,
    pub windowless_rendering_enabled: bool,
    pub external_begin_frame_enabled: bool,
    pub view: *mut c_void,

    #[cfg(target_os = "windows")]
    pub shared_texture_enabled: bool,
}

impl WindowInfo {
    #[must_use]
    pub fn get_cef_window_info(&self) -> cef_window_info_t {
        cef_window_info_t {
            window_name: str_into_cef_string_utf16(&self.window_name),
            bounds: self.bounds.get_cef_rect(),
            hidden: self.hidden.into(),
            parent_view: self.parent_view,
            windowless_rendering_enabled: self.windowless_rendering_enabled.into(),
            external_begin_frame_enabled: self.external_begin_frame_enabled.into(),
            view: self.view,

            #[cfg(target_os = "windows")]
            shared_texture_enabled: self.shared_texture_enabled.into(),
            #[cfg(not(target_os = "windows"))]
            shared_texture_enabled: false.into(),
        }
    }
}
