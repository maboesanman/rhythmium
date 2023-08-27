use cef_sys::cef_resource_bundle_handler_t;

use crate::util::cef_arc::{CefRefCounted, CefRefCountedRaw};

#[repr(transparent)]
pub struct CefResourceBundleHandler(cef_resource_bundle_handler_t);

unsafe impl CefRefCounted for CefResourceBundleHandler {}

unsafe impl CefRefCountedRaw for cef_resource_bundle_handler_t {
    type Wrapper = CefResourceBundleHandler;
}
