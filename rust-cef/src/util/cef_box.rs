use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use cef_sys::cef_base_scoped_t;

use super::cef_type::{CefType, VTable, VTableExt, VTableKindRaw};

/// A box for CEF types.
///
/// These are only created by the crate, and not by the user.
#[repr(transparent)]
pub struct CefBox<T: VTable<Kind = VTableKindBox>> {
    pub(crate) ptr: NonNull<T>,
}

pub struct VTableKindBox;

unsafe impl VTableKindRaw for VTableKindBox {
    type Base = cef_base_scoped_t;

    type Pointer<T: VTable<Kind = Self>> = CefBox<T>;

    type ExtraData = ();

    fn get_initial_extra_data() -> Self::ExtraData {}
}

impl<V: VTable<Kind = VTableKindBox>> Drop for CefBox<V> {
    fn drop(&mut self) {
        unsafe {
            let base = self.ptr.as_ref().get_base();
            match base.del {
                Some(del) => del(self.ptr.as_ptr().cast()),
                None => {}
            }
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
    pub(crate) fn new(mut inner: CefType<V, RustImpl>) -> Self {
        let base = inner.v_table.get_base_mut();
        base.size = std::mem::size_of::<CefType<V, RustImpl>>();
        base.del = Some(del_ptr::<V, RustImpl>);

        let ptr = NonNull::new(Box::into_raw(Box::new(inner))).unwrap();

        Self {
            ptr,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn new_uninit_base() -> cef_base_scoped_t {
    cef_base_scoped_t { size: 0, del: None }
}

// this is only used for types created in rust.
// the drop impl for CefBox calls this via the vtable.
unsafe extern "C" fn del_ptr<V: VTable<Kind = VTableKindBox>, RustImpl>(
    ptr: *mut cef_base_scoped_t,
) {
    _ = Box::from_raw(ptr.cast::<CefType<V, RustImpl>>());
}

impl<V: VTable<Kind = VTableKindBox>> CefBox<V> {
    pub(crate) unsafe fn from_raw(ptr: *mut V) -> Self {
        Self {
            ptr: NonNull::new(ptr).unwrap(),
        }
    }
}
