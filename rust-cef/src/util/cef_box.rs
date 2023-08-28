use std::{ops::Deref, ptr::NonNull};

use cef_sys::cef_base_scoped_t;

use super::cef_base::{CefBase, CefBaseRaw, CefPtrKind};

/// A box for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefBox<B: CefBase<Kind = CefPtrKindBox>, T = ()> {
    ptr: NonNull<CefBoxInner<B, T>>,
}

#[repr(C)]
struct CefBoxInner<B: CefBase<Kind = CefPtrKindBox>, T> {
    base: B,
    data: T,
}

impl<B: CefBase<Kind = CefPtrKindBox>, T> CefBox<B, T> {
    /// Call the delete method from the inner type.
    unsafe fn delete(&mut self) {
        let ptr = B::Kind::get_base(&mut self.ptr.as_mut().base);
        ptr.as_mut().unwrap().del.unwrap()(self.ptr.as_ptr() as *mut _);
    }
}

pub struct CefPtrKindBox;

unsafe impl CefPtrKind for CefPtrKindBox {
    type BaseType = cef_base_scoped_t;

    type Pointer<B: CefBase<Kind = Self>, T> = CefBox<B, T>;

    fn rust_to_ptr<B: CefBase<Kind = Self>, T>(rust: Self::Pointer<B, T>) -> *mut B::CType {
        rust.ptr.as_ptr().cast()
    }

    fn rust_ref_to_ptr<B: CefBase<Kind = Self>, T>(rust: &Self::Pointer<B, T>) -> *mut B::CType {
        rust.ptr.as_ptr().cast()
    }

    fn ptr_to_rust<R: CefBaseRaw<Kind = Self>>(ptr: *mut R) -> Self::Pointer<R::RustType, ()> {
        let ptr = ptr.cast::<CefBoxInner<R::RustType, ()>>();
        let non_null: NonNull<_> = unsafe { ptr.as_ref().unwrap().into() };
        CefBox { ptr: non_null }
    }
}

impl<B: CefBase<Kind = CefPtrKindBox>, T> Drop for CefBox<B, T> {
    fn drop(&mut self) {
        unsafe {
            self.delete();
        }
    }
}

impl<B: CefBase<Kind = CefPtrKindBox>, T> Deref for CefBox<B, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<B: CefBase<Kind = CefPtrKindBox>, T> CefBox<B, T> {
    pub(crate) fn new(base: B, data: T) -> Self {
        let boxed = Box::new(CefBoxInner { base, data });
        let ptr = NonNull::from(&*boxed);
        let mut base = ptr.cast::<cef_base_scoped_t>();

        unsafe {
            let base = base.as_mut();
            base.size = std::mem::size_of::<CefBoxInner<B, T>>();
            base.del = Some(del_ptr::<B, T>);
        }

        Self { ptr }
    }
}

pub(crate) fn new_uninit_base() -> cef_base_scoped_t {
    cef_base_scoped_t { size: 0, del: None }
}

unsafe extern "C" fn del_ptr<B: CefBase<Kind = CefPtrKindBox>, T>(ptr: *mut cef_base_scoped_t) {
    let ptr = ptr.cast::<CefBoxInner<B, T>>();

    let _ = Box::from_raw(ptr);
}
