use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use cef_wrapper::cef_capi_sys::cef_base_scoped_t;

use super::cef_type::{CefType, VTable, VTableExt, VTableKindInternal, CefTypePrefix};

/// A box for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefBox<T: VTable<Kind = VTableKindBox>> {
    pub(crate) ptr: NonNull<T>,
}

pub struct VTableKindBox;

unsafe impl VTableKindInternal for VTableKindBox {
    type Base = cef_base_scoped_t;

    type Pointer<T: VTable<Kind = Self>> = CefBox<T>;

    type ExtraData = ();

    fn into_cef_type<V: VTable<Kind = Self>, R>(capi_v_table: V, rust_impl: R) -> CefType<V, R> {
        let mut capi_v_table = capi_v_table;
        let base = capi_v_table.get_base_mut();
        base.size = std::mem::size_of::<CefType<V, R>>();
        base.del = Some(del_ptr::<CefType<V, R>>);

        let prefix = CefTypePrefix {
            v_table: capi_v_table,
            extra_data: (),
        };

        CefType {
            prefix,
            rust_impl,
        }
    }
}

impl<V: VTable<Kind = VTableKindBox>> Drop for CefBox<V> {
    fn drop(&mut self) {
        unsafe {
            let base = self.ptr.as_ref().get_base();
            base.del.unwrap()(self.ptr.as_ptr() as *mut _);
        }
    }
}

// we can deref to the rust impl if we have a cef type.
// we can't if we only have a vtable.
// this only gets used when implementing traits for cef types.
impl<V: VTable<Kind = VTableKindBox>, RustImpl> Deref for CefBox<CefType<V, RustImpl>> {
    type Target = RustImpl;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().rust_impl }
    }
}

impl<V: VTable<Kind = VTableKindBox>, RustImpl> DerefMut for CefBox<CefType<V, RustImpl>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.ptr.as_mut().rust_impl }
    }
}

impl<V: VTable<Kind = VTableKindBox>, RustImpl> CefBox<CefType<V, RustImpl>> {
    #[allow(dead_code)]
    pub(crate) fn new(inner: CefType<V, RustImpl>) -> Self {
        Self {
            ptr: NonNull::from(&*Box::new(inner)),
        }
    }

    pub(crate) fn type_erase(self) -> CefBox<V> {
        CefBox {
            ptr: self.ptr.cast(),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn new_uninit_base() -> cef_base_scoped_t {
    cef_base_scoped_t { size: 0, del: None }
}

// this is only used for types created in rust.
// the drop impl for CefBox calls this via the vtable.
unsafe extern "C" fn del_ptr<T: VTable<Kind = VTableKindBox>>(
    ptr: *mut cef_base_scoped_t,
) {
    _ = Box::from_raw(ptr.cast::<T>());
}

impl<V: VTable<Kind = VTableKindBox>> CefBox<V> {
    pub(crate) fn into_raw(self) -> *mut V {
        std::mem::ManuallyDrop::new(self).ptr.as_ptr()
    }

    pub(crate) unsafe fn from_raw(ptr: *mut V) -> Self {
        Self {
            ptr: NonNull::new(ptr).unwrap(),
        }
    }
}
