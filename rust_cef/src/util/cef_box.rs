#![allow(dead_code)]

use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use cef_wrapper::cef_capi_sys::cef_base_scoped_t;

use super::starts_with::{StartsWith, StartsWithExt as _};

/// A box for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefBox<T: StartsWith<cef_base_scoped_t>> {
    pub(crate) ptr: NonNull<T>,
}

impl<T: StartsWith<cef_base_scoped_t>> Drop for CefBox<T> {
    fn drop(&mut self) {
        unsafe {
            let base = self.ptr.as_mut().get_start_mut();
            base.del.unwrap()(base);
        }
    }
}

#[repr(C)]
struct CefBoxFromRust<VTable, RustImpl> {
    /// the cef capi type representing the vtable.
    /// this is the first field so that it can be cast to the vtable type by cef.
    /// because this contains the drop function, the RustImpl type can be erased
    /// and still properly dropped.
    capi_v_table: VTable,

    /// the user defined rust type.
    rust_impl: RustImpl,
}

unsafe impl<V, RustImpl> StartsWith<V> for CefBoxFromRust<V, RustImpl> {}
unsafe impl<V: StartsWith<cef_base_scoped_t>, RustImpl> StartsWith<cef_base_scoped_t>
    for CefBoxFromRust<V, RustImpl>
{
}

impl<V: StartsWith<cef_base_scoped_t>, R> CefBoxFromRust<V, R> {
    pub(crate) fn new(mut capi_v_table: V, rust_impl: R) -> Self {
        let base = capi_v_table.get_start_mut();
        base.size = std::mem::size_of::<CefBoxFromRust<V, R>>();
        base.del = Some(del_ptr::<CefBoxFromRust<V, R>>);

        Self {
            capi_v_table,
            rust_impl,
        }
    }

    pub(crate) fn get_rust_impl_from_ptr(ptr: *mut cef_base_scoped_t) -> *mut R {
        let rust_type = unsafe { ptr.cast::<CefBoxFromRust<V, R>>().as_ref().unwrap() };
        &rust_type.rust_impl as *const _ as *mut _
    }
}

// this is only used for types created in rust.
// the drop impl for CefBox calls this via the vtable.
unsafe extern "C" fn del_ptr<T: StartsWith<cef_base_scoped_t>>(ptr: *mut cef_base_scoped_t) {
    unsafe {
        _ = Box::from_raw(ptr.cast::<T>());
    }
}

// we can deref to the rust impl if we have a cef type.
// we can't if we only have a vtable.
// this only gets used when implementing traits for cef types.
impl<V: StartsWith<cef_base_scoped_t>, R> Deref for CefBox<CefBoxFromRust<V, R>> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().rust_impl }
    }
}

// we can deref to the rust impl if we have a cef type.
// we can't if we only have a vtable.
// this only gets used when implementing traits for cef types.
impl<V: StartsWith<cef_base_scoped_t>, R> DerefMut for CefBox<CefBoxFromRust<V, R>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.ptr.as_mut().rust_impl }
    }
}

impl<T: StartsWith<cef_base_scoped_t>> CefBox<T> {
    pub(crate) fn uninit_box_vtable() -> cef_base_scoped_t {
        cef_base_scoped_t { size: 0, del: None }
    }

    pub(crate) fn type_erase<U>(self) -> CefBox<U>
    where
        U: StartsWith<cef_base_scoped_t>,
        T: StartsWith<U>,
    {
        CefBox {
            ptr: self.ptr.cast(),
        }
    }

    pub(crate) fn into_raw(self) -> *mut T {
        std::mem::ManuallyDrop::new(self).ptr.as_ptr()
    }

    pub(crate) unsafe fn from_raw(ptr: *mut T) -> Self {
        unsafe {
            Self {
                ptr: NonNull::new_unchecked(ptr),
            }
        }
    }
}

impl<V: StartsWith<cef_base_scoped_t>, R> CefBox<CefBoxFromRust<V, R>> {
    pub(crate) fn new(capi_v_table: V, rust_impl: R) -> Self {
        let inner = CefBoxFromRust::new(capi_v_table, rust_impl);
        let inner = Box::into_raw(Box::new(inner));
        let ptr = unsafe { NonNull::new_unchecked(inner) };

        Self { ptr }
    }
}
