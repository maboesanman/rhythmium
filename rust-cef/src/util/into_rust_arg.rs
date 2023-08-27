use cef_sys::cef_string_utf16_t;

use super::{cef_base::{CefBaseRaw, CefPtrKind, CefBase}, cef_arc::{CefArc, CefPtrKindArc}, cef_box::{CefBox, CefPtrKindBox}};

pub(crate) trait IntoRustArg {
    type RustType;

    fn into_rust_arg(self) -> Self::RustType;
}

pub(crate) trait IntoRustArgRef {
    type RustType;

    unsafe fn into_rust_arg_ref<'a>(self) -> &'a mut Self::RustType;
}

pub(crate) trait IntoCArg {
    type CType;

    fn into_c_arg(self) -> *mut Self::CType;
}


impl<R: CefBaseRaw> IntoRustArg for *mut R
{
    type RustType = <R::Kind as CefPtrKind>::Pointer<R::RustType>;

    fn into_rust_arg(self) -> Self::RustType {
        <R::Kind as CefPtrKind>::ptr_to_rust(self)
    }
}

impl<R: CefBaseRaw> IntoRustArgRef for *mut R
{
    type RustType = R::RustType;

    unsafe fn into_rust_arg_ref<'a>(self) -> &'a mut Self::RustType {
        self.cast::<Self::RustType>().as_mut().unwrap()
    }
}

impl<T: CefBase<Kind=CefPtrKindArc>> IntoCArg for CefArc<T>
{
    type CType = T::CType;

    fn into_c_arg(self) -> *mut Self::CType {
        <T::Kind as CefPtrKind>::rust_to_ptr(self)
    }
}

impl<T: CefBase<Kind=CefPtrKindBox>> IntoCArg for CefBox<T>
{
    type CType = T::CType;

    fn into_c_arg(self) -> *mut Self::CType {
        <T::Kind as CefPtrKind>::rust_to_ptr(self)
    }
}

impl IntoRustArg for *const cef_string_utf16_t {
    type RustType = String;

    fn into_rust_arg(self) -> Self::RustType {
        let slice = unsafe {
            let cef_string = self.as_ref().unwrap();
            std::slice::from_raw_parts(cef_string.str_, cef_string.length)
        };
    
        String::from_utf16_lossy(slice)
    }
}
