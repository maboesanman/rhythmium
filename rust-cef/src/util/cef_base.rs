use std::ops::Deref;

use super::{cef_arc::CefPtrKindArc, cef_box::CefPtrKindBox};

/// A marker trait for vtables of cef types.
///
/// This should only be impmented for types from the CEF C API.
pub unsafe trait CefBase {
    type CType: CefBaseRaw<RustType = Self, Kind = Self::Kind>;
    type Kind: CefPtrKind;
}

pub unsafe trait CefBaseRaw {
    type RustType: CefBase<CType = Self, Kind = Self::Kind>;
    type Kind: CefPtrKind;
}

pub unsafe trait CefPtrKind {
    type BaseType;

    type Pointer<B: CefBase<Kind = Self>, T>: Deref<Target = T>;

    fn get_base<B: CefBase<Kind = Self>>(ptr: *mut B) -> *mut Self::BaseType {
        ptr.cast()
    }

    fn rust_to_ptr<B: CefBase<Kind = Self>, T>(rust: Self::Pointer<B, T>) -> *mut B::CType;

    fn rust_ref_to_ptr<B: CefBase<Kind = Self>, T>(rust: &Self::Pointer<B, T>) -> *mut B::CType;

    fn ptr_to_rust<R: CefBaseRaw<Kind = Self>>(ptr: *mut R) -> Self::Pointer<R::RustType, ()>;
}

pub trait CefArcBase: CefBase<Kind = CefPtrKindArc> {}

impl<T: CefBase<Kind = CefPtrKindArc>> CefArcBase for T {}

pub trait CefBoxBase: CefBase<Kind = CefPtrKindBox> {}

impl<T: CefBase<Kind = CefPtrKindBox>> CefBoxBase for T {}
