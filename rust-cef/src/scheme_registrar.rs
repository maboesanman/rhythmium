use cef_sys::cef_scheme_registrar_t;

use crate::{
    scheme_options::SchemeOptions,
    util::{
        cef_box::{CefBox, VTableKindBox},
        cef_string::into_cef_str_utf16,
        cef_type::VTable,
    },
};

#[repr(transparent)]
pub struct SchemeRegistrar(cef_scheme_registrar_t);

unsafe impl VTable for SchemeRegistrar {
    type Kind = VTableKindBox;
}

impl CefBox<SchemeRegistrar> {
    pub fn add_custom_scheme(
        &self,
        scheme_names: &str,
        options: impl Into<SchemeOptions>,
    ) -> Result<(), CustomSchemeRegistrationError> {
        let scheme_names = into_cef_str_utf16(scheme_names);
        let options = options.into().bits() as std::os::raw::c_int;

        let result = unsafe { invoke_v_table!(self.add_custom_scheme(&scheme_names, options)) };

        if result == 1 {
            Ok(())
        } else {
            Err(CustomSchemeRegistrationError)
        }
    }
}

#[derive(Debug)]
pub struct CustomSchemeRegistrationError;
