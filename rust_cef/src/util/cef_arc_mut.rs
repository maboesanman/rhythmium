use std::{ops::{Deref, DerefMut}, ptr::NonNull};

use super::{cef_type::{VTable, CefType}, cef_arc::{VTableKindArc, CefArc}};


#[repr(transparent)]
pub struct CefArcMut<T: VTable<Kind = VTableKindArc>>(pub(crate) CefArc<T>);

// we can deref to the rust impl if we have a cef type.
// we can't if we only have a vtable.
// this only gets used when implementing traits for cef types.
impl<V: VTable<Kind = VTableKindArc>, RustImpl> Deref for CefArcMut<CefType<V, RustImpl>> {
    type Target = RustImpl;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.0.ptr.as_ref().rust_impl }
    }
}

// we can deref to the rust impl if we have a cef type.
// we can't if we only have a vtable.
// this only gets used when implementing traits for cef types.
impl<V: VTable<Kind = VTableKindArc>, RustImpl> DerefMut for CefArcMut<CefType<V, RustImpl>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.0.ptr.as_mut().rust_impl }
    }
}

impl<V: VTable<Kind = VTableKindArc>, RustImpl> CefArcMut<CefType<V, RustImpl>> {
    pub(crate) fn new(inner: CefType<V, RustImpl>) -> Self {
        Self(CefArc {
            ptr: NonNull::from(&*Box::new(inner)),
        })
    }

    pub(crate) fn type_erase(self) -> CefArcMut<V> {
        CefArcMut(self.0.type_erase())
    }
}
