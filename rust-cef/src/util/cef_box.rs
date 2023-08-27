use std::{marker::PhantomData, ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

use cef_sys::cef_base_scoped_t;


/// A marker trait for types that are reference counted by CEF.
///
/// This should only be impmented for types from the CEF C API.
pub unsafe trait CefScoped {}

pub(crate) unsafe trait CefScopedRaw {
    type Wrapper: CefScoped;
}

/// A box for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefBox<T: CefScoped> {
    ptr: NonNull<T>,
}

unsafe impl<T: CefScoped> Send for CefBox<T> where T: Send {}
unsafe impl<T: CefScoped> Sync for CefBox<T> where T: Sync {}


fn wrap_boolean(b: bool) -> i32 {
    if b {
        1
    } else {
        0
    }
}

impl<T: CefScoped> CefBox<T> {
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

impl<T: CefScoped> Drop for CefBox<T> {
    fn drop(&mut self) {
        unsafe {
            self.delete();
        }
    }
}

impl<T: CefScoped> Deref for CefBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref() }
    }
}

impl<T: CefScoped> CefBox<T> {
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

    pub(crate) fn from_ptr<W>(ptr: *mut W) -> Self
    where
        W: CefScopedRaw<Wrapper = T>,
    {
        let ptr = ptr.cast::<T>();
        let non_null: NonNull<_> = unsafe { ptr.as_ref().unwrap().into() };
        Self { ptr: non_null }
    }

    pub(crate) fn into_ptr<W>(self) -> *mut W
    where
        W: CefScopedRaw<Wrapper = T>,
    {
        self.ptr.as_ptr().cast()
    }
}

pub(crate) fn new_uninit_base() -> cef_base_scoped_t {
    cef_base_scoped_t {
        size: 0,
        del: None,
    }
}

unsafe extern "C" fn del_ptr<T: CefScoped>(ptr: *mut cef_base_scoped_t) {
    let ptr = ptr.cast::<T>();

    let _ = Box::from_raw(ptr);
}
