use cef_wrapper::{cef_capi_sys::{cef_base_ref_counted_t, cef_browser_t}};

use crate::util::{cef_arc::CefArc, starts_with::StartsWith};

use super::browser_host::BrowserHost;

#[repr(transparent)]
pub struct Browser(pub(crate) cef_browser_t);

unsafe impl StartsWith<cef_browser_t> for Browser {}
unsafe impl StartsWith<cef_base_ref_counted_t> for Browser {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_browser_t {}

impl Browser {
    pub fn get_host(&self) -> CefArc<BrowserHost> {
        let ptr = &self.0 as *const _ as *mut _;
        unsafe {
            let host_t = self.0.get_host.unwrap()(ptr);
            CefArc::from_raw(host_t.cast())
        }
    }
}
