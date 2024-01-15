#[derive(Debug)]
pub struct BrowserHost {
    pub(crate) browser_host: *mut crate::cef_capi_sys::cef_browser_host_t,
}

impl BrowserHost {
    pub(crate) fn new(browser_host: *mut crate::cef_capi_sys::cef_browser_host_t) -> Self {
        Self { browser_host }
    }

    pub fn was_resized(&self) {
        unsafe {
            (*self.browser_host).was_resized.unwrap()(self.browser_host.cast());
        }
    }

    pub fn notify_screen_info_changed(&self) {
        unsafe {
            (*self.browser_host).notify_screen_info_changed.unwrap()(self.browser_host.cast());
        }
    }
}

impl Drop for BrowserHost {
    fn drop(&mut self) {
        unsafe {
            (*self.browser_host).base.release.unwrap()(self.browser_host.cast());
        }
    }
}
