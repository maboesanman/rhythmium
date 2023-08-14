use std::{marker::PhantomData, ptr::NonNull, sync::atomic::AtomicUsize, ops::Deref};

use cef_sys::cef_base_ref_counted_t;

#[cfg(not(sanitize = "thread"))]
macro_rules! acquire {
    ($x:expr) => {
        std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire)
    };
}

// ThreadSanitizer does not support memory fences. To avoid false positive
// reports in Arc / Weak implementation use atomic loads for synchronization
// instead.
#[cfg(sanitize = "thread")]
macro_rules! acquire {
    ($x:expr) => {
        $x.load(Acquire)
    };
}

/// A marker trait for types that are reference counted by CEF.
///
/// This should only be impmented for types from the CEF C API.
pub unsafe trait CefRefCounted {}

pub unsafe trait CefRefCountedRaw {
    type Wrapper: CefRefCounted;
}

/// A reference counted wrapper for CEF types.
/// 
/// These are only created by the crate, and not by the user.
pub struct CefArc<T: CefRefCounted> {
    ptr: NonNull<CefArcInner<T>>,
}

#[repr(C)]
struct CefArcInner<T: CefRefCounted> {
    data: T,
    ref_count: AtomicUsize,
}

fn wrap_boolean(b: bool) -> i32 {
    if b {
        1
    } else {
        0
    }
}

impl<T: CefRefCounted> CefArcInner<T> {
    fn get_base(&self) -> &cef_base_ref_counted_t {
        unsafe {
            let base = NonNull::from(&self.data);
            let base: NonNull<cef_base_ref_counted_t> = base.cast();
            base.as_ref()
        }
    }

    /// Increment the reference count.
    fn add_ref(&self) {
        unsafe { self.get_base().add_ref.unwrap()(self as *const _ as *mut _) };
    }

    /// Decrement the reference count.
    fn release(&self) {
        unsafe { self.get_base().release.unwrap()(self as *const _ as *mut _) };
    }
}



impl<T: CefRefCounted> Clone for CefArc<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.ptr.as_ref().add_ref();
        }
        Self {
            ptr: self.ptr,
        }
    }
}

impl<T: CefRefCounted> Drop for CefArc<T> {
    fn drop(&mut self) {
        unsafe {
            self.ptr.as_ref().release();
        }
    }
}

impl<T: CefRefCounted> Deref for CefArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: CefRefCounted> CefArc<T> {
    pub(crate) fn new(inner: T) -> Self {
        let inner = Box::leak(Box::new(CefArcInner {
            data: inner,
            ref_count: AtomicUsize::new(1),
        }));

        let base = NonNull::from(&inner.data);
        let mut base: NonNull<cef_base_ref_counted_t> = base.cast();
        unsafe {
            base.as_mut().size = std::mem::size_of_val(inner);
            base.as_mut().add_ref = Some(add_ref_ptr::<T>);
            base.as_mut().release = Some(release_ptr::<T>);
            base.as_mut().has_one_ref = Some(has_one_ref_ptr::<T>);
            base.as_mut().has_at_least_one_ref = Some(has_at_least_one_ref_ptr::<T>);
        }

        Self {
            ptr: inner.into(),
        }
    }

    pub(crate) fn from_ptr<W>(ptr: *mut W) -> Self
    where W: CefRefCountedRaw<Wrapper = T>
    {
        let ptr = ptr.cast::<CefArcInner<T>>();
        let non_null: NonNull<_> = unsafe {
            ptr.as_ref().unwrap().into()
        };
        Self {
            ptr: non_null
        }
    }
}

pub(crate) fn new_uninit_base() -> cef_base_ref_counted_t {
    cef_base_ref_counted_t {
        size: 0,
        add_ref: None,
        release: None,
        has_one_ref: None,
        has_at_least_one_ref: None,
    }
}

unsafe extern "C" fn add_ref_ptr<T: CefRefCounted>(ptr: *mut cef_base_ref_counted_t) {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = unsafe { inner.as_ref() };
    let inner = inner.unwrap();
    inner.ref_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

unsafe extern "C" fn release_ptr<T: CefRefCounted>(ptr: *mut cef_base_ref_counted_t) -> i32 {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = unsafe { inner.as_ref() };
    let inner = inner.unwrap();
    if inner.ref_count.fetch_sub(1, std::sync::atomic::Ordering::Release) != 0 {
        return 0
    }

    acquire!(self.ref_count);

    unsafe {
        let _ = Box::from_raw(inner as *const _ as *mut CefArcInner<T>);
    }

    1
}

unsafe extern "C" fn has_one_ref_ptr<T: CefRefCounted>(ptr: *mut cef_base_ref_counted_t) -> i32 {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = unsafe { inner.as_ref() };
    let inner = inner.unwrap();
    wrap_boolean(inner.ref_count.load(std::sync::atomic::Ordering::Acquire) == 1)
}

unsafe extern "C" fn has_at_least_one_ref_ptr<T: CefRefCounted>(ptr: *mut cef_base_ref_counted_t) -> i32 {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = unsafe { inner.as_ref() };
    let inner = inner.unwrap();
    wrap_boolean(inner.ref_count.load(std::sync::atomic::Ordering::Acquire) >= 1)
}
