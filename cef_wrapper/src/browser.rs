

#[derive(Debug)]
pub struct Browser {
    pub(crate) browser: *mut crate::sys::cef_browser_t,
}

impl Browser {
    pub(crate) fn new(browser: *mut crate::sys::cef_browser_t) -> Self {
        Self { browser }
    }

    pub fn get_host(&self) -> crate::browser_host::BrowserHost {
        unsafe {
            crate::browser_host::BrowserHost::new(
                (*self.browser).get_host.unwrap()(self.browser.cast())
            )
        }
    }

    pub fn is_valid(&self) -> bool {
        unsafe { (*self.browser).is_valid.unwrap()(self.browser.cast()) != 0 }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        unsafe {
            (*self.browser).base.release.unwrap()(self.browser.cast());
        }
    }
}