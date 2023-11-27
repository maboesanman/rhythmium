#[repr(C)]
pub struct CefType<V: VTable, RustImpl> {
    pub(crate) v_table: V,
    pub(crate) extra_data: <V::Kind as VTableKindRaw>::ExtraData,
    pub(crate) rust_impl: RustImpl,
}

unsafe impl<V: VTable, RustType> VTable for CefType<V, RustType> {
    type Kind = V::Kind;
}

impl<V: VTable, RustImpl> CefType<V, RustImpl> {
    pub(crate) fn new(v_table: V, rust_impl: RustImpl) -> Self {
        Self {
            v_table,
            extra_data: <V::Kind as VTableKindRaw>::get_initial_extra_data(),
            rust_impl,
        }
    }
}

/// This trait marks a type as a vtable compatible with CEF.
/// it is implemented on the vtables from cef and on the user defined rust
/// types that start with vtables.
/// It can be either an arc based or box based vtable.
///
/// # Safety
///
/// This trait must only be implemented on properly constructed cef vtables.
pub unsafe trait VTable: Sized {
    type Kind: VTableKind;
}

pub(crate) trait VTableExt: VTable + Sized {
    fn get_base(&self) -> &<Self::Kind as VTableKindRaw>::Base {
        let self_ptr = self as *const Self;
        let base_ptr = self_ptr.cast::<<Self::Kind as VTableKindRaw>::Base>();
        unsafe { &*base_ptr }
    }

    fn get_base_mut(&mut self) -> &mut <Self::Kind as VTableKindRaw>::Base {
        let self_ptr = self as *mut Self;
        let base_ptr = self_ptr.cast::<<Self::Kind as VTableKindRaw>::Base>();
        unsafe { &mut *base_ptr }
    }
}

impl<V: VTable> VTableExt for V {}

/// This trait marks a type as a vtable compatible with CEF.
#[allow(private_bounds)]
pub trait VTableKind: VTableKindRaw {}

/// # Safety
///
/// This trait must only be implemented on properly constructed cef vtables.
pub(crate) unsafe trait VTableKindRaw {
    type Base;

    type Pointer<T: VTable<Kind = Self>>;

    type ExtraData;

    fn get_initial_extra_data() -> Self::ExtraData;
}

impl<T: VTableKindRaw> VTableKind for T {}
