use std::{marker::PhantomData, ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

use cef_sys::cef_base_scoped_t;

use super::cef_base::{CefBase, CefPtrKind, CefBaseRaw};

/// A box for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefBox<T: CefBase<Kind=CefPtrKindBox>> {
    ptr: NonNull<T>,
}

unsafe impl<T: CefBase<Kind=CefPtrKindBox>> Send for CefBox<T> where T: Send {}
unsafe impl<T: CefBase<Kind=CefPtrKindBox>> Sync for CefBox<T> where T: Sync {}

fn wrap_boolean(b: bool) -> i32 {
    if b {
        1
    } else {
        0
    }
}

impl<T: CefBase<Kind=CefPtrKindBox>> CefBox<T> {
    fn get_base(&self) -> &cef_base_scoped_t {
        unsafe {
            let base: NonNull<cef_base_scoped_t> = self.ptr.cast();
            base.as_ref()
        }
    }

    /// Call the delete method from the inner type.
    unsafe fn delete(&mut self) {
        self.get_base().del.unwrap()(self.ptr.as_ptr() as *mut _);
    }
}

pub struct CefPtrKindBox;

unsafe impl CefPtrKind for CefPtrKindBox {
    type Pointer<T: CefBase<Kind=Self>> = CefBox<T>;

    fn rust_to_ptr<B: CefBase<Kind=Self>>(rust: Self::Pointer<B>) -> *mut B::CType {
        rust.ptr.as_ptr().cast()
    }

    fn ptr_to_rust<R: CefBaseRaw<Kind=Self>>(ptr: *mut R) -> Self::Pointer<R::RustType> {
        let ptr = ptr.cast::<R::RustType>();
        let non_null: NonNull<_> = unsafe { ptr.as_ref().unwrap().into() };
        CefBox { ptr: non_null }
    }
}

impl<T: CefBase<Kind=CefPtrKindBox>> Drop for CefBox<T> {
    fn drop(&mut self) {
        unsafe {
            self.delete();
        }
    }
}

impl<T: CefBase<Kind=CefPtrKindBox>> Deref for CefBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref() }
    }
}

impl<T: CefBase<Kind=CefPtrKindBox>> CefBox<T> {
    pub(crate) fn new(inner: T) -> Self {
        let boxed = Box::new(inner);
        let ptr = NonNull::from(&*boxed);
        let mut base = ptr.cast::<cef_base_scoped_t>();

        unsafe {
            let base = base.as_mut();
            base.size = std::mem::size_of::<T>();
            base.del = Some(del_ptr::<T>);
        }

        Self { ptr }
    }
}

pub(crate) fn new_uninit_base() -> cef_base_scoped_t {
    cef_base_scoped_t { size: 0, del: None }
}

unsafe extern "C" fn del_ptr<T: CefBase<Kind=CefPtrKindBox>>(ptr: *mut cef_base_scoped_t) {
    let ptr = ptr.cast::<T>();

    let _ = Box::from_raw(ptr);
}
