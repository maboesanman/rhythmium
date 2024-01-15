use cef_wrapper::cef_capi_sys::{cef_base_ref_counted_t, cef_browser_host_t};

use crate::util::starts_with::StartsWith;

#[repr(transparent)]
pub struct BrowserHost(pub(crate) cef_browser_host_t);

unsafe impl StartsWith<cef_browser_host_t> for BrowserHost {}
unsafe impl StartsWith<cef_base_ref_counted_t> for BrowserHost {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_browser_host_t {}

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
