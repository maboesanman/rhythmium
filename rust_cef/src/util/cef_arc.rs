use std::{ops::Deref, ptr::NonNull, sync::Arc};

use super::starts_with::{StartsWith, StartsWithExt as _};
use cef_wrapper::cef_capi_sys::cef_base_ref_counted_t;
use std::fmt::Debug;

/// A reference counted wrapper for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefArc<T: StartsWith<cef_base_ref_counted_t>> {
    pub(crate) ptr: NonNull<T>,
}

impl<T: Debug + StartsWith<cef_base_ref_counted_t>> Debug for CefArc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = unsafe { self.ptr.as_ref() };
        f.debug_struct("CefArc").field("inner", &inner).finish()
    }
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
            rust_impl,
        }
    }

    pub(crate) fn get_rust_impl_from_ptr(ptr: *mut cef_base_ref_counted_t) -> *mut R {
        let rust_type = unsafe { ptr.cast::<CefArcFromRust<V, R>>().as_ref().unwrap() };
        &rust_type.rust_impl as *const _ as *mut _
    }
}

mod c_callbacks {
    use std::sync::Arc;

    use cef_wrapper::cef_capi_sys::cef_base_ref_counted_t;

    use crate::util::wrap_boolean::wrap_boolean;

    use super::CefArcFromRust;

    pub unsafe extern "C" fn add_ref_ptr<V, R>(ptr: *mut cef_base_ref_counted_t) {
        let rust_type = ptr.cast::<CefArcFromRust<V, R>>();
        Arc::increment_strong_count(rust_type);
    }

    pub unsafe extern "C" fn release_ptr<V, R>(ptr: *mut cef_base_ref_counted_t) -> i32 {
        let rust_type = ptr.cast::<CefArcFromRust<V, R>>();
        let a = Arc::from_raw(rust_type);
        let strong_count = Arc::strong_count(&a);
        drop(a);

        wrap_boolean(strong_count == 1)
    }

    pub unsafe extern "C" fn has_one_ref_ptr<V, R>(ptr: *mut cef_base_ref_counted_t) -> i32 {
        let rust_type = ptr.cast::<CefArcFromRust<V, R>>();
        let a = Arc::from_raw(rust_type);
        let strong_count = Arc::strong_count(&a);
        core::mem::forget(a);

        wrap_boolean(strong_count == 1)
    }

    pub unsafe extern "C" fn has_at_least_one_ref_ptr<V, R>(
        ptr: *mut cef_base_ref_counted_t,
    ) -> i32 {
        let rust_type = ptr.cast::<CefArcFromRust<V, R>>();
        let a = Arc::from_raw(rust_type);
        let strong_count = Arc::strong_count(&a);
        core::mem::forget(a);

        wrap_boolean(strong_count >= 1)
    }
}

impl<T: StartsWith<cef_base_ref_counted_t>> Deref for CefArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
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
            ptr: self.into_non_null().cast(),
        }
    }

    pub(crate) fn into_raw(self) -> *mut T {
        self.into_non_null().as_ptr()
    }

    fn into_non_null(self) -> NonNull<T> {
        std::mem::ManuallyDrop::new(self).ptr
    }

    pub(crate) unsafe fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }

    pub fn try_get_mut(&mut self) -> Result<&mut T, &T> {
        let base = unsafe { self.ptr.as_ref().get_start() };
        if unsafe { base.has_one_ref.unwrap()(base as *const _ as *mut _) } != 0 {
            Ok(unsafe { self.ptr.as_mut() })
        } else {
            Err(unsafe { self.ptr.as_ref() })
        }
    }
}

impl<V: StartsWith<cef_base_ref_counted_t>, R> CefArc<CefArcFromRust<V, R>> {
    pub(crate) fn new(capi_v_table: V, rust_impl: R) -> Self {
        let inner = CefArcFromRust::new(capi_v_table, rust_impl);
        let inner = Arc::into_raw(Arc::new(inner));
        let ptr = unsafe { NonNull::new_unchecked(inner as *mut _) };

        Self { ptr }
    }
}
