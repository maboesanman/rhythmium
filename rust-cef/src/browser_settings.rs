use std::os::raw::c_int;

use cef_sys::{cef_browser_settings_t, cef_state_t};

use crate::{color::Color, util::cef_string::str_into_cef_string_utf16};

#[derive(Debug, Clone, Default)]
pub struct BrowserSettings {
    pub windowless_frame_rate: u32,
    pub standard_font_family: Option<String>,
    pub fixed_font_family: Option<String>,
    pub serif_font_family: Option<String>,
    pub sans_serif_font_family: Option<String>,
    pub cursive_font_family: Option<String>,
    pub fantasy_font_family: Option<String>,
    pub default_font_size: u32,
    pub default_fixed_font_size: u32,
    pub minimum_font_size: u32,
    pub minimum_logical_font_size: u32,
    pub default_encoding: Option<String>,
    pub remote_fonts: Option<bool>,
    pub javascript: Option<bool>,
    pub javascript_close_windows: Option<bool>,
    pub javascript_access_clipboard: Option<bool>,
    pub javascript_dom_paste: Option<bool>,
    pub image_loading: Option<bool>,
    pub image_shrink_standalone_to_fit: Option<bool>,
    pub text_area_resize: Option<bool>,
    pub tab_to_links: Option<bool>,
    pub local_storage: Option<bool>,
    pub databases: Option<bool>,
    pub webgl: Option<bool>,
    pub background_color: Color,
    pub chrome_status_bubble: Option<bool>,
    pub chrome_zoom_bubble: Option<bool>,
}

fn opt_bool_to_cef_state(b: Option<bool>) -> cef_state_t {
    match b {
        None => 0,
        Some(true) => 1,
        Some(false) => 2,
    }
}

impl BrowserSettings {
    pub fn get_cef_browser_settings(&self) -> cef_browser_settings_t {
        cef_browser_settings_t {
            size: std::mem::size_of::<cef_browser_settings_t>(),
            windowless_frame_rate: self.windowless_frame_rate as c_int,
            standard_font_family: str_into_cef_string_utf16(
                self.standard_font_family.as_deref().unwrap_or(""),
            ),
            fixed_font_family: str_into_cef_string_utf16(
                self.fixed_font_family.as_deref().unwrap_or(""),
            ),
            serif_font_family: str_into_cef_string_utf16(
                self.serif_font_family.as_deref().unwrap_or(""),
            ),
            sans_serif_font_family: str_into_cef_string_utf16(
                self.sans_serif_font_family.as_deref().unwrap_or(""),
            ),
            cursive_font_family: str_into_cef_string_utf16(
                self.cursive_font_family.as_deref().unwrap_or(""),
            ),
            fantasy_font_family: str_into_cef_string_utf16(
                self.fantasy_font_family.as_deref().unwrap_or(""),
            ),
            default_font_size: self.default_font_size as c_int,
            default_fixed_font_size: self.default_fixed_font_size as c_int,
            minimum_font_size: self.minimum_font_size as c_int,
            minimum_logical_font_size: self.minimum_logical_font_size as c_int,
            default_encoding: str_into_cef_string_utf16(
                self.default_encoding.as_deref().unwrap_or(""),
            ),
            remote_fonts: opt_bool_to_cef_state(self.remote_fonts),
            javascript: opt_bool_to_cef_state(self.javascript),
            javascript_close_windows: opt_bool_to_cef_state(self.javascript_close_windows),
            javascript_access_clipboard: opt_bool_to_cef_state(self.javascript_access_clipboard),
            javascript_dom_paste: opt_bool_to_cef_state(self.javascript_dom_paste),
            image_loading: opt_bool_to_cef_state(self.image_loading),
            image_shrink_standalone_to_fit: opt_bool_to_cef_state(
                self.image_shrink_standalone_to_fit,
            ),
            text_area_resize: opt_bool_to_cef_state(self.text_area_resize),
            tab_to_links: opt_bool_to_cef_state(self.tab_to_links),
            local_storage: opt_bool_to_cef_state(self.local_storage),
            databases: opt_bool_to_cef_state(self.databases),
            webgl: opt_bool_to_cef_state(self.webgl),
            background_color: self.background_color,
            chrome_status_bubble: opt_bool_to_cef_state(self.chrome_status_bubble),
            chrome_zoom_bubble: opt_bool_to_cef_state(self.chrome_zoom_bubble),
        }
    }
}
