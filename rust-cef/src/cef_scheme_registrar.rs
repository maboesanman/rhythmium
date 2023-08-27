use cef_sys::cef_scheme_registrar_t;

use crate::util::cef_arc::{CefRefCounted, CefRefCountedRaw};

#[repr(transparent)]
pub struct CefSchemeRegistrar(cef_scheme_registrar_t);

unsafe impl CefRefCounted for CefSchemeRegistrar {}

unsafe impl CefRefCountedRaw for cef_scheme_registrar_t {
    type Wrapper = CefSchemeRegistrar;
}
