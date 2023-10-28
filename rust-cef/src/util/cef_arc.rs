use std::{ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

use cef_sys::cef_base_ref_counted_t;

use super::{
    wrap_boolean::wrap_boolean, cef_type::{CefType, VTableKind, VTable, VTableExt},
};

/// A reference counted wrapper for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefArc<T: VTable<Kind=VtableKindArc>> {
    ptr: NonNull<T>,
}

pub struct VtableKindArc;

unsafe impl VTableKind for VtableKindArc {
    type Base = cef_base_ref_counted_t;

    type Pointer<V: VTable<Kind = Self>> = CefArc<V>;

    type ExtraData = CefArcExtraData;

    fn into_rust<V: VTable<Kind = Self>>(vtable: *const V) -> Self::Pointer<V> {
        let non_null: NonNull<V> = unsafe { vtable.as_ref().unwrap().into() };
        CefArc { ptr: non_null }
    }
}

pub struct CefArcExtraData {
    ref_count: AtomicUsize,
}

impl<V: VTable<Kind=VtableKindArc>> Clone for CefArc<V> {
    fn clone(&self) -> Self {
        unsafe {
            let base = self.ptr.as_ref().get_base();
            base.add_ref.unwrap()(self as *const _ as *mut _);
        }
        Self { ptr: self.ptr }
    }
}

impl<V: VTable<Kind=VtableKindArc>> Drop for CefArc<V> {
    fn drop(&mut self) {
        unsafe {
            let base = self.ptr.as_ref().get_base();
            base.release.unwrap()(self as *const _ as *mut _);
        }
    }
}

// we can deref to the rust impl if we have a cef type.
// we can't if we only have a vtable.
// this only gets used when implementing traits for cef types.
impl<V: VTable<Kind=VtableKindArc>, RustImpl> Deref for CefArc<CefType<V, RustImpl>> {
    type Target = RustImpl;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().rust_impl }
    }
}

impl<V: VTable<Kind=VtableKindArc>, RustImpl> CefArc<CefType<V, RustImpl>> {
    pub(crate) fn new(mut inner: CefType<V, RustImpl>) -> Self {
        let base = inner.v_table.get_base_mut();
        base.size = std::mem::size_of::<CefType<V, RustImpl>>();
        base.add_ref = Some(add_ref_ptr::<V, RustImpl>);
        base.release = Some(release_ptr::<V, RustImpl>);
        base.has_one_ref = Some(has_one_ref_ptr::<V, RustImpl>);
        base.has_at_least_one_ref = Some(has_at_least_one_ref_ptr::<V, RustImpl>);

        Self { ptr: NonNull::from(&*Box::new(inner)) }
    }

    // fn has_one_ref(&self) -> bool {
    //     unsafe {
    //         let mut base = self.ptr.cast::<cef_base_ref_counted_t>();
    //         base.as_mut().has_one_ref.unwrap()(base.as_mut()) != 0
    //     }
    // }

    // fn has_at_least_one_ref(&self) -> bool {
    //     unsafe {
    //         let mut base = self.ptr.cast::<cef_base_ref_counted_t>();
    //         base.as_mut().has_at_least_one_ref.unwrap()(base.as_mut()) != 0
    //     }
    // }
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

unsafe extern "C" fn add_ref_ptr<V: VTable<Kind=VtableKindArc>, RustImpl> (
    ptr: *mut cef_base_ref_counted_t,
) {
    let inner = ptr.cast::<CefType<V, RustImpl>>();
    let inner = inner.as_ref().unwrap();
    inner
        .extra_data
        .ref_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

unsafe extern "C" fn release_ptr<V: VTable<Kind=VtableKindArc>, RustImpl> (
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let inner = ptr.cast::<CefType<V, RustImpl>>();
    let inner = inner.as_ref().unwrap();
    if inner
        .extra_data
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

    // we know this box came from rust_cef, so it is a CefType<V, RustImpl>.
    _ = Box::from_raw(ptr.cast::<CefType<V, RustImpl>>());

    1
}

unsafe extern "C" fn has_one_ref_ptr<V: VTable<Kind=VtableKindArc>, RustImpl>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let inner = ptr.cast::<CefType<V, RustImpl>>();
    let inner = inner.as_ref().unwrap();
    wrap_boolean(inner.extra_data.ref_count.load(std::sync::atomic::Ordering::Acquire) == 1)
}

unsafe extern "C" fn has_at_least_one_ref_ptr<V: VTable<Kind=VtableKindArc>, RustImpl>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let inner = ptr.cast::<CefType<V, RustImpl>>();
    let inner = inner.as_ref().unwrap();
    wrap_boolean(inner.extra_data.ref_count.load(std::sync::atomic::Ordering::Acquire) >= 1)
}
