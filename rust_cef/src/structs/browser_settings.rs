use std::num::NonZeroU32;

use cef_wrapper::cef_capi_sys::{
    cef_browser_settings_t, cef_state_t_STATE_DEFAULT, cef_state_t_STATE_DISABLED,
    cef_state_t_STATE_ENABLED,
};

use crate::util::cef_string::str_into_cef_string_utf16;

#[derive(Default)]
pub struct BrowserSettings {
    pub windowless_frame_rate: Option<NonZeroU32>,

    pub standard_font_family: Option<String>,
    pub fixed_font_family: Option<String>,
    pub serif_font_family: Option<String>,
    pub sans_serif_font_family: Option<String>,
    pub cursive_font_family: Option<String>,
    pub fantasy_font_family: Option<String>,
    pub default_font_size: Option<u32>,
    pub default_fixed_font_size: Option<u32>,
    pub minimum_font_size: Option<u32>,
    pub minimum_logical_font_size: Option<u32>,

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
    pub background_color: u32,
    pub chrome_status_bubble: Option<bool>,
    pub chrome_zoom_bubble: Option<bool>,
}

impl From<&BrowserSettings> for cef_browser_settings_t {
    fn from(value: &BrowserSettings) -> Self {
        let wrap_string =
            { |s: &Option<String>| str_into_cef_string_utf16(s.as_deref().unwrap_or("")) };

        let wrap_bool = |b: &Option<bool>| match b {
            Some(true) => cef_state_t_STATE_ENABLED,
            Some(false) => cef_state_t_STATE_DISABLED,
            None => cef_state_t_STATE_DEFAULT,
        };

        cef_browser_settings_t {
            size: std::mem::size_of::<cef_browser_settings_t>(),
            windowless_frame_rate: value.windowless_frame_rate.map(|v| v.get()).unwrap_or(0) as i32,
            standard_font_family: wrap_string(&value.standard_font_family),
            fixed_font_family: wrap_string(&value.fixed_font_family),
            serif_font_family: wrap_string(&value.serif_font_family),
            sans_serif_font_family: wrap_string(&value.sans_serif_font_family),
            cursive_font_family: wrap_string(&value.cursive_font_family),
            fantasy_font_family: wrap_string(&value.fantasy_font_family),
            default_font_size: value.default_font_size.unwrap_or(0) as i32,
            default_fixed_font_size: value.default_fixed_font_size.unwrap_or(0) as i32,
            minimum_font_size: value.minimum_font_size.unwrap_or(0) as i32,
            minimum_logical_font_size: value.minimum_logical_font_size.unwrap_or(0) as i32,
            default_encoding: wrap_string(&value.default_encoding),
            remote_fonts: wrap_bool(&value.remote_fonts),
            javascript: wrap_bool(&value.javascript),
            javascript_close_windows: wrap_bool(&value.javascript_close_windows),
            javascript_access_clipboard: wrap_bool(&value.javascript_access_clipboard),
            javascript_dom_paste: wrap_bool(&value.javascript_dom_paste),
            image_loading: wrap_bool(&value.image_loading),
            image_shrink_standalone_to_fit: wrap_bool(&value.image_shrink_standalone_to_fit),
            text_area_resize: wrap_bool(&value.text_area_resize),
            tab_to_links: wrap_bool(&value.tab_to_links),
            local_storage: wrap_bool(&value.local_storage),
            databases: wrap_bool(&value.databases),
            webgl: wrap_bool(&value.webgl),
            background_color: value.background_color,
            chrome_status_bubble: wrap_bool(&value.chrome_status_bubble),
            chrome_zoom_bubble: wrap_bool(&value.chrome_zoom_bubble),
        }
    }
}
