use std::{
    ops::Deref,
    ptr::NonNull,
    sync::atomic::AtomicUsize,
};

use cef_wrapper::cef_capi_sys::cef_base_ref_counted_t;

use super::{
    cef_type::{CefType, VTable, VTableExt, VTableKindInternal, CefTypePrefix},
    wrap_boolean::wrap_boolean, cef_arc_mut::CefArcMut,
};

/// A reference counted wrapper for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefArc<T: VTable<Kind = VTableKindArc>> {
    pub(crate) ptr: NonNull<T>,
}

unsafe impl<T: VTable<Kind = VTableKindArc>> Send for CefArc<T> {}
unsafe impl<T: VTable<Kind = VTableKindArc>> Sync for CefArc<T> {}

impl<T: VTable<Kind = VTableKindArc>> CefArc<T> {
    pub fn try_into_mut(self) -> Result<CefArcMut<T>, Self> {
        let base = unsafe { self.ptr.as_ref().get_base() };
        if unsafe { base.has_one_ref.unwrap()(base as *const _ as *mut _) } != 0 {
            Ok(CefArcMut(self))
        } else {
            Err(self)
        }
    }
}

pub struct VTableKindArc;

unsafe impl VTableKindInternal for VTableKindArc {
    type Base = cef_base_ref_counted_t;

    type Pointer<V: VTable<Kind = Self>> = CefArc<V>;

    type ExtraData = CefArcExtraData;

    fn into_cef_type<V: VTable<Kind = Self>, R>(capi_v_table: V, rust_impl: R) -> CefType<V, R> {
        let mut capi_v_table = capi_v_table;
        let base = capi_v_table.get_base_mut();
        base.size = std::mem::size_of::<CefType<V, R>>();
        base.add_ref = Some(add_ref_ptr::<CefType<V, R>>);
        base.release = Some(release_ptr::<CefType<V, R>>);
        base.has_one_ref = Some(has_one_ref_ptr::<CefType<V, R>>);
        base.has_at_least_one_ref = Some(has_at_least_one_ref_ptr::<CefType<V, R>>);

        let prefix = CefTypePrefix {
            v_table: capi_v_table,
            extra_data: Self::ExtraData {
                ref_count: AtomicUsize::new(1),
            },
        };

        CefType {
            prefix,
            rust_impl,
        }
    }
}

pub struct CefArcExtraData {
    ref_count: AtomicUsize,
}

impl<V: VTable<Kind = VTableKindArc>> Clone for CefArc<V> {
    fn clone(&self) -> Self {
        unsafe {
            let base = self.ptr.as_ref().get_base();
            base.add_ref.unwrap()(self as *const _ as *mut _);
        }
        Self { ptr: self.ptr }
    }
}

impl<V: VTable<Kind = VTableKindArc>> Drop for CefArc<V> {
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
impl<V: VTable<Kind = VTableKindArc>, RustImpl> Deref for CefArc<CefType<V, RustImpl>> {
    type Target = RustImpl;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().rust_impl }
    }
}

impl<V: VTable<Kind = VTableKindArc>> From<CefArcMut<V>> for CefArc<V> {
    fn from(value: CefArcMut<V>) -> Self {
        value.0
    }
}

#[allow(dead_code)]
pub(crate) fn new_uninit_base() -> cef_base_ref_counted_t {
    cef_base_ref_counted_t {
        size: 0,
        add_ref: None,
        release: None,
        has_one_ref: None,
        has_at_least_one_ref: None,
    }
}

unsafe extern "C" fn add_ref_ptr<T: VTable<Kind = VTableKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) {
    let prefix = ptr.cast::<T>().as_ref().unwrap().get_prefix();
    prefix
        .extra_data
        .ref_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

unsafe extern "C" fn release_ptr<T: VTable<Kind = VTableKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let prefix = ptr.cast::<T>().as_ref().unwrap().get_prefix();
    if prefix
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

    // we know this box came from rust_cef, so it is a CefType<V>.
    _ = Box::from_raw(ptr.cast::<T>());

    1
}

unsafe extern "C" fn has_one_ref_ptr<T: VTable<Kind = VTableKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let prefix = ptr.cast::<T>().as_ref().unwrap().get_prefix();
    wrap_boolean(
        prefix
            .extra_data
            .ref_count
            .load(std::sync::atomic::Ordering::Acquire)
            == 1,
    )
}

unsafe extern "C" fn has_at_least_one_ref_ptr<T: VTable<Kind = VTableKindArc>>(
    ptr: *mut cef_base_ref_counted_t,
) -> i32 {
    let prefix = ptr.cast::<T>().as_ref().unwrap().get_prefix();
    wrap_boolean(
        prefix
            .extra_data
            .ref_count
            .load(std::sync::atomic::Ordering::Acquire)
            >= 1,
    )
}


impl<V: VTable<Kind = VTableKindArc>, RustImpl> CefArc<CefType<V, RustImpl>> {
    pub(crate) fn new(inner: CefType<V, RustImpl>) -> Self {
        CefArcMut::new(inner).into()
    }

    pub(crate) fn type_erase(self) -> CefArc<V> {
        CefArc {
            ptr: self.ptr.cast(),
        }
    }
}

impl<V: VTable<Kind = VTableKindArc>> CefArc<V> {
    pub(crate) fn into_raw(self) -> *mut V {
        std::mem::ManuallyDrop::new(self).ptr.as_ptr()
    }

    pub(crate) unsafe fn from_raw(ptr: *mut V) -> Self {
        Self {
            ptr: NonNull::new(ptr).unwrap(),
        }
    }
}
