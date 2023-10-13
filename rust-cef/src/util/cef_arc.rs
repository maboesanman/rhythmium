use std::{ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

use cef_sys::cef_base_ref_counted_t;

use super::{
    wrap_boolean::wrap_boolean, cef_type::CefType,
};

/// A reference counted wrapper for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefArc<T: CefType<Kind = CefPtrKindArc>> {
    ptr: NonNull<CefArcInner<T>>,
}

pub struct CefPtrKindArc;

unsafe impl CefPtrKind for CefPtrKindArc {
    type BaseType = cef_base_ref_counted_t;

    type Pointer<T: CefBase<Kind = Self>> = CefArc<T>;

    fn rust_to_ptr<T: CefBase<Kind = Self>>(rust: Self::Pointer<T>) -> *mut T::CType {
        rust.ptr.as_ptr().cast()
    }

    fn rust_ref_to_ptr<T: CefBase<Kind = Self>>(rust: &Self::Pointer<T>) -> *mut T::CType {
        rust.ptr.as_ptr().cast()
    }

    fn ptr_to_rust<R: CefBaseRaw<Kind = Self>>(ptr: *mut R) -> Self::Pointer<R::RustType> {
        let non_null: NonNull<_> = unsafe { ptr.as_ref().unwrap().into() };
        let ptr = non_null.cast::<CefArcInner<R::RustType>>();
        CefArc { ptr }
    }
}

#[repr(C)]
struct CefArcInner<T: CefType<Kind = CefPtrKindArc>> {
    inner: T,
    ref_count: AtomicUsize,
}

impl<T: CefType<Kind = CefPtrKindArc>> CefArcInner<T> {
    fn get_base(&self) -> &cef_base_ref_counted_t {
        unsafe {
            T::Kind::get_base(&self.inner as *const _ as *mut T)
                .as_ref()
                .unwrap()
        }
    }

    /// Increment the reference count.
    fn add_ref(&self) {
        unsafe { self.get_base().add_ref.unwrap()(self as *const _ as *mut _) };
    }

    /// Decrement the reference count, freeing the object if it reaches zero.
    fn release(&self) {
        unsafe { self.get_base().release.unwrap()(self as *const _ as *mut _) };
    }
}

impl<T: CefType<Kind = CefPtrKindArc>> Clone for CefArc<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.ptr.as_ref().add_ref();
        }
        Self { ptr: self.ptr }
    }
}

impl<T: CefType<Kind = CefPtrKindArc>> Drop for CefArc<T> {
    fn drop(&mut self) {
        unsafe {
            self.ptr.as_ref().release();
        }
    }
}

// impl<T: CefType<Kind = CefPtrKindArc>> Deref for CefArc<T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         unsafe { &self.ptr.as_ref().data }
//     }
// }

impl<CType, RustImpl> Deref for CefArc<CefType<CType, RustImpl>> {
    type Target = RustImpl;

    fn deref(&self) -> &Self::Target {
        let inner = unsafe { self.ptr.as_ref() };

        &inner.inner.rust_impl
    }
}

impl<T: CefType<Kind = CefPtrKindArc>> CefArc<T> {
    pub(crate) fn new(inner: T) -> Self {
        let inner = Box::leak(Box::new(CefArcInner {
            inner,
            ref_count: AtomicUsize::new(1),
        }));

        let base = NonNull::from(&inner.inner);
        let mut base: NonNull<cef_base_ref_counted_t> = base.cast();
        unsafe {
            base.as_mut().size = std::mem::size_of_val(inner);
            base.as_mut().add_ref = Some(add_ref_ptr::<T>);
            base.as_mut().release = Some(release_ptr::<T>);
            base.as_mut().has_one_ref = Some(has_one_ref_ptr::<T>);
            base.as_mut().has_at_least_one_ref = Some(has_at_least_one_ref_ptr::<T>);
        }

        Self { ptr: inner.into() }
    }

    pub(crate) fn get_base(&self) -> &T::CType {
        let inner = unsafe { &self.ptr.as_ref() };

        inner.inner.get_v_table()
    }

    fn has_one_ref(&self) -> bool {
        unsafe {
            let mut base = self.ptr.cast::<cef_base_ref_counted_t>();
            base.as_mut().has_one_ref.unwrap()(base.as_mut()) != 0
        }
    }

    fn has_at_least_one_ref(&self) -> bool {
        unsafe {
            let mut base = self.ptr.cast::<cef_base_ref_counted_t>();
            base.as_mut().has_at_least_one_ref.unwrap()(base.as_mut()) != 0
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

unsafe extern "C" fn add_ref_ptr<T: CefType<Kind = CefPtrKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = inner.as_ref().unwrap();
    inner
        .ref_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

unsafe extern "C" fn release_ptr<T: CefType<Kind = CefPtrKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = inner.as_ref().unwrap();
    if inner
        .ref_count
        .fetch_sub(1, std::sync::atomic::Ordering::Release)
        != 0
    {
        return 0;
    }

    // this sanitize thing is just lifted from the standard library arc implementation.
    #[cfg(not(sanitize = "thread"))]
    std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);

    #[cfg(sanitize = "thread")]
    inner.ref_count.load(std::sync::atomic::Ordering::Acquire);

    // we know this box came from rust_cef, so it is a CefArcInner.
    let _ = Box::from_raw(inner as *const _ as *mut CefArcInner<T>);

    1
}

unsafe extern "C" fn has_one_ref_ptr<T: CefType<Kind = CefPtrKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = inner.as_ref().unwrap();
    wrap_boolean(inner.ref_count.load(std::sync::atomic::Ordering::Acquire) == 1)
}

unsafe extern "C" fn has_at_least_one_ref_ptr<T: CefType<Kind = CefPtrKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let inner = ptr.cast::<CefArcInner<T>>();
    let inner = inner.as_ref().unwrap();
    wrap_boolean(inner.ref_count.load(std::sync::atomic::Ordering::Acquire) >= 1)
}
