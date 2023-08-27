use cef_sys::cef_scheme_registrar_t;

use crate::util::{cef_base::{CefBase, CefBaseRaw}, cef_box::CefPtrKindBox};

#[repr(transparent)]
pub struct CefSchemeRegistrar(cef_scheme_registrar_t);

unsafe impl CefBase for CefSchemeRegistrar {
    type CType = cef_scheme_registrar_t;
    type Kind = CefPtrKindBox;
}

unsafe impl CefBaseRaw for cef_scheme_registrar_t {
    type RustType = CefSchemeRegistrar;
    type Kind = CefPtrKindBox;
}
