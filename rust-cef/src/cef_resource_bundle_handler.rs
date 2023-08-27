use cef_sys::cef_resource_bundle_handler_t;

use crate::util::{cef_base::{CefBase, CefBaseRaw}, cef_arc::CefPtrKindArc};

#[repr(transparent)]
pub struct CefResourceBundleHandler(cef_resource_bundle_handler_t);

unsafe impl CefBase for CefResourceBundleHandler {
    type CType = cef_resource_bundle_handler_t;
    type Kind = CefPtrKindArc;
}

unsafe impl CefBaseRaw for cef_resource_bundle_handler_t {
    type RustType = CefResourceBundleHandler;
    type Kind = CefPtrKindArc;
}
