use std::num::NonZeroU32;

use cef_wrapper::cef_capi_sys::cef_browser_settings_t;

use crate::{enums::state::State, util::cef_string::str_into_cef_string_utf16};

#[derive(Default)]
pub struct BrowserSettings {
    // pub size: Size,
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

    pub remote_fonts: State,

    pub javascript: State,

    pub javascript_close_windows: State,

    pub javascript_access_clipboard: State,

    pub javascript_dom_paste: State,

    pub image_loading: State,

    pub image_shrink_standalone_to_fit: State,

    pub text_area_resize: State,

    pub tab_to_links: State,

    pub local_storage: State,

    pub databases: State,

    pub webgl: State,

    pub background_color: u32,

    pub chrome_status_bubble: State,

    pub chrome_zoom_bubble: State,
}

impl From<&BrowserSettings> for cef_browser_settings_t {
    fn from(value: &BrowserSettings) -> Self {
        let wrap_string =
            |s: &Option<String>| str_into_cef_string_utf16(s.as_deref().unwrap_or(""));

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
            remote_fonts: value.remote_fonts.into(),
            javascript: value.javascript.into(),
            javascript_close_windows: value.javascript_close_windows.into(),
            javascript_access_clipboard: value.javascript_access_clipboard.into(),
            javascript_dom_paste: value.javascript_dom_paste.into(),
            image_loading: value.image_loading.into(),
            image_shrink_standalone_to_fit: value.image_shrink_standalone_to_fit.into(),
            text_area_resize: value.text_area_resize.into(),
            tab_to_links: value.tab_to_links.into(),
            local_storage: value.local_storage.into(),
            databases: value.databases.into(),
            webgl: value.webgl.into(),
            background_color: value.background_color,
            chrome_status_bubble: value.chrome_status_bubble.into(),
            chrome_zoom_bubble: value.chrome_zoom_bubble.into(),
        }
    }
}
