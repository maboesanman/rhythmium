use cef_sys::cef_audio_parameters_t;

// this is used as the rust impl for dyn variants of cef types.

pub struct Unknown;

#[repr(C)]
pub struct CefType<VTable, RustImpl> {
    pub(crate) v_table: VTable,
    pub(crate) rust_impl: RustImpl,
}



unsafe impl<V: VTable> VTable for CefType<V, Unknown> {
    type Kind = V::Kind;
}

impl<V: VTable, RustImpl> CefType<V, RustImpl> {
    pub fn get_v_table(&self) -> &V {
        &self.v_table
    }

    pub fn get_v_table_base(&self) -> &<V::Kind as VTableKind>::Base {
        self.v_table.get_base()
    }
}

// impl<V: VTable>

/// This trait marks a type as a vtable from CEF.
/// It can be either an arc based or box based vtable.
pub unsafe trait VTable {
    type Kind: VTableKind;
}

trait VTableExt: VTable + Sized {
    fn get_base(&self) -> &<Self::Kind as VTableKind>::Base {
        unsafe { &*Self::get_base_raw(self) }
    }

    fn get_base_raw(self: *const Self) -> *const <Self::Kind as VTableKind>::Base {
        self.cast()
    }

    fn into_rust(self: *const Self) -> <Self::Kind as VTableKind>::Pointer<CefType<Self, Unknown>> {
        Self::Kind::into_rust(self)
    }
}

impl<V: VTable> VTableExt for V {}

pub unsafe trait VTableKind {
    type Base;

    type Pointer<T>;

    fn into_rust<V: VTable<Kind=Self>>(vtable: *const V) -> Self::Pointer<CefType<V, Unknown>>;
}

