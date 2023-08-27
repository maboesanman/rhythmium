use std::ops::Deref;

use super::{cef_arc::CefArc, cef_box::CefBox};


/// A marker trait for smart pointers that cross the ffi boundary.
///
/// This should only be impmented for types from the CEF C API.
pub unsafe trait CefBaseRaw: Sized {
    type RustType: CefBase<Kind=Self::Kind, CType = Self>;
    type Kind: CefPtrKind;
}

pub unsafe trait CefBase {
    type CType: CefBaseRaw<RustType = Self>;
    type Kind: CefPtrKind;
}

pub unsafe trait CefPtrKind {
    #[doc(hidden)]
    type Pointer<T: CefBase<Kind=Self>>: Deref<Target = T>;

    #[doc(hidden)]
    fn rust_to_ptr<B: CefBase<Kind=Self>>(rust: Self::Pointer<B>) -> *mut B::CType;

    #[doc(hidden)]
    fn ptr_to_rust<R: CefBaseRaw<Kind=Self>>(ptr: *mut R) -> Self::Pointer<R::RustType>;
}
