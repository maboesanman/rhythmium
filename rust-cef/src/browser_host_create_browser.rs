use cef_sys::cef_browser_host_create_browser;

use crate::{
    browser_settings::BrowserSettings,
    client::Client,
    util::{cef_arc::CefArc, cef_string::str_into_cef_string_utf16},
    window_info::WindowInfo,
};

pub fn browser_host_create_browser(
    window_info: &WindowInfo,
    client: CefArc<Client>,
    url: &str,
    settings: &BrowserSettings,
) -> Result<(), i32> {
    let window_info = window_info.get_cef_window_info();
    let client = client.into_raw().cast();
    let url = str_into_cef_string_utf16(url);
    let settings = settings.get_cef_browser_settings();
    let result = unsafe {
        cef_browser_host_create_browser(
            &window_info,
            client,
            &url,
            &settings,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if result != 0 {
        Err(result)
    } else {
        Ok(())
    }
}
