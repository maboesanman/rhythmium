#[repr(C)]
pub struct CefType<V: VTable, RustImpl> {
    pub(crate) v_table: V,
    pub(crate) extra_data: <V::Kind as VTableKind>::ExtraData,
    pub(crate) rust_impl: RustImpl,
}

unsafe impl<V: VTable, RustType> VTable for CefType<V, RustType> {
    type Kind = V::Kind;
}

impl<V: VTable, RustImpl> CefType<V, RustImpl> {
    pub(crate) fn new(v_table: V, rust_impl: RustImpl) -> Self {
        Self {
            v_table,
            extra_data: <V::Kind as VTableKind>::get_initial_extra_data(),
            rust_impl,
        }
    }
    pub fn get_v_table(&self) -> &V {
        &self.v_table
    }

    pub fn get_v_table_mut(&mut self) -> &mut V {
        &mut self.v_table
    }

    pub fn get_v_table_base(&self) -> &<V::Kind as VTableKind>::Base {
        self.v_table.get_base()
    }

    pub fn get_v_table_base_mut(&mut self) -> &mut <V::Kind as VTableKind>::Base {
        self.v_table.get_base_mut()
    }
}

/// This trait marks a type as a vtable compatible with CEF.
/// it is implemented on the vtables from cef and on the user defined rust
/// types that start with vtables.
/// It can be either an arc based or box based vtable.
pub unsafe trait VTable {
    type Kind: VTableKind;
}

pub trait VTableExt: VTable + Sized {
    fn get_base(&self) -> &<Self::Kind as VTableKind>::Base {
        let self_ptr = self as *const Self;
        let base_ptr = self_ptr.cast::<<Self::Kind as VTableKind>::Base>();
        unsafe { &*base_ptr }
    }

    fn get_base_mut(&mut self) -> &mut <Self::Kind as VTableKind>::Base {
        let self_ptr = self as *mut Self;
        let base_ptr = self_ptr.cast::<<Self::Kind as VTableKind>::Base>();
        unsafe { &mut *base_ptr }
    }
}

impl<V: VTable> VTableExt for V {}

pub unsafe trait VTableKind {
    type Base;

    type Pointer<T: VTable<Kind = Self>>;

    type ExtraData;

    fn get_initial_extra_data() -> Self::ExtraData;
}
