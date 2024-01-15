use std::{ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

use super::starts_with::{StartsWith, StartsWithExt as _};
use cef_wrapper::cef_capi_sys::cef_base_ref_counted_t;

/// A reference counted wrapper for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefArc<T: StartsWith<cef_base_ref_counted_t>> {
    pub(crate) ptr: NonNull<T>,
}

unsafe impl<T: StartsWith<cef_base_ref_counted_t>> Send for CefArc<T> {}
unsafe impl<T: StartsWith<cef_base_ref_counted_t>> Sync for CefArc<T> {}

impl<T: StartsWith<cef_base_ref_counted_t>> Drop for CefArc<T> {
    fn drop(&mut self) {
        unsafe {
            let base = self.ptr.as_ref().get_start();
            base.release.unwrap()(base as *const _ as *mut _);
        }
    }
}

impl<T: StartsWith<cef_base_ref_counted_t>> Clone for CefArc<T> {
    fn clone(&self) -> Self {
        unsafe {
            let base = self.ptr.as_ref().get_start();
            base.add_ref.unwrap()(base as *const _ as *mut _);
        }
        Self { ptr: self.ptr }
    }
}

/// CefType is a type from rust that has been prepared to be sent to cef.
/// It contains a vtable and the user defined rust type, as well as any
/// extra data needed by the smart pointer (a ref count for CefArc, nothing for CefBox).
#[repr(C)]
pub struct CefArcFromRust<VTable, RustImpl> {
    /// the cef capi type representing the vtable.
    /// this is the first field so that it can be cast to the vtable type by cef.
    /// because this contains the drop function, the RustImpl type can be erased
    /// and still properly dropped.
    pub(crate) capi_v_table: VTable,

    /// extra data needed by the smart pointer.
    pub(crate) ref_count: AtomicUsize,

    /// the user defined rust type.
    pub rust_impl: RustImpl,
}

unsafe impl<V, R> StartsWith<V> for CefArcFromRust<V, R> {}
unsafe impl<V: StartsWith<cef_base_ref_counted_t>, R> StartsWith<cef_base_ref_counted_t>
    for CefArcFromRust<V, R>
{
}

impl<V: StartsWith<cef_base_ref_counted_t>, R> CefArcFromRust<V, R> {
    /// capi_v_table is the partially completed vtable.
    /// the values in the base will be populatef by this function.
    pub(crate) fn new(mut capi_v_table: V, rust_impl: R) -> Self {
        let base = capi_v_table.get_start_mut();
        base.size = std::mem::size_of::<CefArcFromRust<V, R>>();
        base.add_ref = Some(c_callbacks::add_ref_ptr::<V, R>);
        base.release = Some(c_callbacks::release_ptr::<V, R>);
        base.has_one_ref = Some(c_callbacks::has_one_ref_ptr::<V, R>);
        base.has_at_least_one_ref = Some(c_callbacks::has_at_least_one_ref_ptr::<V, R>);

        Self {
            capi_v_table,
            ref_count: AtomicUsize::new(1),
            rust_impl,
        }
    }

    pub(crate) fn get_rust_impl_from_ptr(ptr: *mut cef_base_ref_counted_t) -> *mut R {
        let rust_type = unsafe { ptr.cast::<CefArcFromRust<V, R>>().as_ref().unwrap() };
        &rust_type.rust_impl as *const _ as *mut _
    }
}

mod c_callbacks {
    use cef_wrapper::cef_capi_sys::cef_base_ref_counted_t;

    use crate::util::wrap_boolean::wrap_boolean;

    use super::CefArcFromRust;

    pub unsafe extern "C" fn add_ref_ptr<V, R>(ptr: *mut cef_base_ref_counted_t) {
        let rust_type = ptr.cast::<CefArcFromRust<V, R>>().as_ref().unwrap();
        rust_type
            .ref_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub unsafe extern "C" fn release_ptr<V, R>(ptr: *mut cef_base_ref_counted_t) -> i32 {
        let rust_type = ptr.cast::<CefArcFromRust<V, R>>().as_ref().unwrap();
        if rust_type
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
        _ = Box::from_raw(ptr.cast::<CefArcFromRust<V, R>>());

        1
    }

    pub unsafe extern "C" fn has_one_ref_ptr<V, R>(ptr: *mut cef_base_ref_counted_t) -> i32 {
        let rust_type = ptr.cast::<CefArcFromRust<V, R>>().as_ref().unwrap();
        wrap_boolean(
            rust_type
                .ref_count
                .load(std::sync::atomic::Ordering::Acquire)
                == 1,
        )
    }

    pub unsafe extern "C" fn has_at_least_one_ref_ptr<V, R>(
        ptr: *mut cef_base_ref_counted_t,
    ) -> i32 {
        let prefix = ptr.cast::<CefArcFromRust<V, R>>().as_ref().unwrap();
        wrap_boolean(prefix.ref_count.load(std::sync::atomic::Ordering::Acquire) >= 1)
    }
}

// we can deref to the rust impl if we have a cef type.
// we can't if we only have a vtable.
// this only gets used when implementing traits for cef types.
impl<V: StartsWith<cef_base_ref_counted_t>, R> Deref for CefArc<CefArcFromRust<V, R>> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().rust_impl }
    }
}

pub(crate) fn uninit_arc_vtable() -> cef_base_ref_counted_t {
    cef_base_ref_counted_t {
        size: 0,
        add_ref: None,
        release: None,
        has_one_ref: None,
        has_at_least_one_ref: None,
    }
}

impl<T: StartsWith<cef_base_ref_counted_t>> CefArc<T> {
    pub(crate) fn type_erase<U>(self) -> CefArc<U>
    where
        U: StartsWith<cef_base_ref_counted_t>,
        T: StartsWith<U>,
    {
        CefArc {
            ptr: self.ptr.cast(),
        }
    }

    pub(crate) fn into_raw(self) -> *mut T {
        std::mem::ManuallyDrop::new(self).ptr.as_ptr()
    }

    pub(crate) unsafe fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }
}

// impl<T: StartsWith<cef_base_ref_counted_t>> CefArc<T> {
//     pub fn try_into_mut(self) -> Result<CefArcMut<T>, Self> {
//         let base = unsafe { self.ptr.as_ref().get_start() };
//         if unsafe { base.has_one_ref.unwrap()(base as *const _ as *mut _) } != 0 {
//             Ok(CefArcMut(self))
//         } else {
//             Err(self)
//         }
//     }
// }

// impl<V: VTable<Kind = VTableKindArc>> From<CefArcMut<V>> for CefArc<V> {
//     fn from(value: CefArcMut<V>) -> Self {
//         value.0
//     }
// }

impl<V: StartsWith<cef_base_ref_counted_t>, R> CefArc<CefArcFromRust<V, R>> {
    pub(crate) fn new(capi_v_table: V, rust_impl: R) -> Self {
        let inner = CefArcFromRust::new(capi_v_table, rust_impl);
        let inner = Box::into_raw(Box::new(inner));
        let ptr = unsafe { NonNull::new_unchecked(inner) };

        Self { ptr }
    }
}
