use cef_wrapper::cef_capi_sys::{self, cef_base_ref_counted_t, cef_browser_host_t, cef_client_t};
use std::ptr;

use crate::{
    rust_to_c::client::Client,
    structs::{browser_settings::BrowserSettings, window_info::WindowInfo},
    util::{cef_arc::CefArc, cef_string::str_into_cef_string_utf16, starts_with::StartsWith},
};
use std::fmt::{Debug, Formatter};

#[repr(transparent)]
pub struct BrowserHost(pub(crate) cef_browser_host_t);

unsafe impl StartsWith<cef_browser_host_t> for BrowserHost {}
unsafe impl StartsWith<cef_base_ref_counted_t> for BrowserHost {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_browser_host_t {}

impl Debug for BrowserHost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserHost").finish()
    }
}

impl BrowserHost {
    pub fn was_resized(&self) {
        let ptr = &self.0 as *const _ as *mut _;
        unsafe {
            self.0.was_resized.unwrap()(ptr);
        }
    }

    pub fn notify_screen_info_changed(&self) {
        let ptr = &self.0 as *const _ as *mut _;
        unsafe {
            self.0.notify_screen_info_changed.unwrap()(ptr);
        }
    }
}

impl BrowserHost {
    pub fn create_browser(
        window_info: &WindowInfo,
        client: CefArc<Client>,
        url: &str,
        browser_settings: &BrowserSettings,
        // extra_info
        // request_context
    ) -> bool {
        let window_info = window_info.into();
        let client = client.type_erase::<cef_client_t>().into_raw();
        let url = str_into_cef_string_utf16(url);
        let browser_settings = browser_settings.into();
        let result = unsafe {
            cef_capi_sys::cef_browser_host_create_browser(
                &window_info,
                client,
                &url,
                &browser_settings,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        result != 0
    }
}
