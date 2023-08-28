use cef_sys::cef_string_utf16_t;

use super::{
    cef_arc::{CefArc, CefPtrKindArc},
    cef_base::{CefBase, CefBaseRaw, CefPtrKind},
    cef_box::{CefBox, CefPtrKindBox},
};

pub(crate) trait IntoRustArg {
    type RustType;

    fn into_rust_arg(self) -> Self::RustType;
}

pub(crate) trait IntoRustArgRef {
    type RustType<T>;

    unsafe fn into_rust_arg_ref<'a, T>(self) -> &'a Self::RustType<T>;
}

pub(crate) trait IntoCArg {
    type CType;

    fn into_c_arg(self) -> Self::CType;
}

pub(crate) trait IntoCArgRef {
    type CType;

    fn into_c_arg_ref(&self) -> *mut Self::CType;
}

impl<R: CefBaseRaw> IntoRustArg for *mut R {
    type RustType = <R::Kind as CefPtrKind>::Pointer<R::RustType, ()>;

    fn into_rust_arg(self) -> Self::RustType {
        <R::Kind as CefPtrKind>::ptr_to_rust(self)
    }
}

impl<R: CefBaseRaw> IntoRustArgRef for *mut R {
    type RustType<T> = <R::Kind as CefPtrKind>::Pointer<R::RustType, T>;

    unsafe fn into_rust_arg_ref<'a, T>(self) -> &'a Self::RustType<T> {
        self.cast::<Self::RustType<T>>().as_mut().unwrap()
    }
}

impl<B: CefBase<Kind = CefPtrKindArc>, T> IntoCArg for CefArc<B, T> {
    type CType = *mut B::CType;

    fn into_c_arg(self) -> Self::CType {
        <B::Kind as CefPtrKind>::rust_to_ptr(self)
    }
}

impl<B: CefBase<Kind = CefPtrKindArc>, T> IntoCArgRef for CefArc<B, T> {
    type CType = B::CType;

    fn into_c_arg_ref(&self) -> *mut Self::CType {
        <B::Kind as CefPtrKind>::rust_ref_to_ptr(self)
    }
}

impl<B: CefBase<Kind = CefPtrKindBox>, T> IntoCArg for CefBox<B, T> {
    type CType = *mut B::CType;

    fn into_c_arg(self) -> Self::CType {
        <B::Kind as CefPtrKind>::rust_to_ptr(self)
    }
}

impl<B: CefBase<Kind = CefPtrKindBox>, T> IntoCArgRef for CefBox<B, T> {
    type CType = B::CType;

    fn into_c_arg_ref(&self) -> *mut Self::CType {
        <B::Kind as CefPtrKind>::rust_ref_to_ptr(self)
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

impl IntoCArg for &str {
    type CType = cef_string_utf16_t;

    fn into_c_arg(self) -> Self::CType {
        let bytes = self.encode_utf16().collect::<Vec<_>>();
        let bytes = bytes.into_boxed_slice();

        let (str_, length) = Box::into_raw(bytes).to_raw_parts();

        let str_ = str_.cast();

        unsafe extern "C" fn drop_string(ptr: *mut u16) {
            todo!()
        }

        cef_string_utf16_t {
            str_,
            length,
            dtor: Some(drop_string),
        }
    }
}
