use std::{ops::Deref, ptr::NonNull};

use cef_sys::cef_base_scoped_t;

/// A box for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefBox<T: CefBase<Kind = CefPtrKindBox>> {
    ptr: NonNull<T>,
}

impl<T: CefBase<Kind = CefPtrKindBox>> CefBox<T> {
    /// Call the delete method from the inner type.
    unsafe fn delete(&mut self) {
        let ptr = T::Kind::get_base(self.ptr.as_ptr());
        ptr.as_mut().unwrap().del.unwrap()(self.ptr.as_ptr() as *mut _);
    }
}

pub struct CefPtrKindBox;

unsafe impl CefPtrKind for CefPtrKindBox {
    type BaseType = cef_base_scoped_t;

    type Pointer<T: CefBase<Kind = Self>> = CefBox<T>;

    fn rust_to_ptr<T: CefBase<Kind = Self>>(rust: Self::Pointer<T>) -> *mut T::CType {
        rust.ptr.as_ptr().cast()
    }

    fn rust_ref_to_ptr<T: CefBase<Kind = Self>>(rust: &Self::Pointer<T>) -> *mut T::CType {
        rust.ptr.as_ptr().cast()
    }

    fn ptr_to_rust<R: CefBaseRaw<Kind = Self>>(ptr: *mut R) -> Self::Pointer<R::RustType> {
        let ptr = ptr.cast::<R::RustType>();
        let non_null: NonNull<_> = unsafe { ptr.as_ref().unwrap().into() };
        CefBox { ptr: non_null }
    }
}

impl<T: CefBase<Kind = CefPtrKindBox>> Drop for CefBox<T> {
    fn drop(&mut self) {
        unsafe {
            self.delete();
        }
    }
}

impl<T: CefBase<Kind = CefPtrKindBox>> Deref for CefBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref() }
    }
}

impl<T: CefBase<Kind = CefPtrKindBox>> CefBox<T> {
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

unsafe extern "C" fn del_ptr<T: CefBase<Kind = CefPtrKindBox>>(ptr: *mut cef_base_scoped_t) {
    let ptr = ptr.cast::<T>();

    let _ = Box::from_raw(ptr);
}
