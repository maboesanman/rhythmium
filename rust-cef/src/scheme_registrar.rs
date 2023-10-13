use std::ffi::c_uint;

use cef_sys::cef_scheme_registrar_t;

use crate::{
    scheme_options::CefSchemeOptions,
    util::{
        cef_base::{CefBase, CefBaseRaw},
        cef_box::{CefBox, CefPtrKindBox},
        into_rust_arg::{IntoCArg, IntoCArgRef},
    },
};

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

impl CefBox<CefSchemeRegistrar> {
    pub fn add_custom_scheme(
        &self,
        scheme_names: &str,
        options: CefSchemeOptions,
    ) -> Result<(), ()> {
        let this = self.into_c_arg_ref();
        let scheme_names = scheme_names.into_c_arg();
        let options: c_uint = options.into();
        let options = options as i32;

        match unsafe { (*this).add_custom_scheme.unwrap()(this, &scheme_names, options) == 1 } {
            true => Ok(()),
            false => Err(()),
        }
    }
}
