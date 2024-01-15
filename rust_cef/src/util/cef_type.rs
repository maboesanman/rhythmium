use super::cef_arc::VTableKindArc;



/// CefType is a type from rust that has been prepared to be sent to cef.
/// It contains a vtable and the user defined rust type, as well as any 
/// extra data needed by the smart pointer (a ref count for CefArc, nothing for CefBox).
#[repr(C)]
pub struct CefType<V: VTable, RustImpl> {
    pub(crate) prefix: CefTypePrefix<V>,

    /// the user defined rust type.
    pub(crate) rust_impl: RustImpl,
}

/// CefTypePrefix is the part of CefType that is shared between all rust-originating
/// types. It contains the vtable and the extra data needed by the rust implementation
/// of the smart pointer.
pub struct CefTypePrefix<V: VTable> {
    /// the cef capi type representing the vtable.
    /// this is the first field so that it can be cast to the vtable type by cef.
    /// because this contains the drop function, cef arc and cef box can erase the
    /// rust impl and still drop custom types.
    pub(crate) v_table: V,

    /// extra data needed by the smart pointer.
    pub(crate) extra_data: <V::Kind as VTableKindInternal>::ExtraData,
}

/// # Safety
/// this is only safe to implement on types that start with either a
/// cef_base_ref_counted_t or a cef_base_scoped_t, and which have their size and
/// destructor set correctly.
unsafe impl<V: VTable, RustType> VTable for CefType<V, RustType> {
    type Kind = V::Kind;
}

/// # Safety
/// this is only safe to implement on types that start with either a
/// cef_base_ref_counted_t or a cef_base_scoped_t, and which have their size and
/// destructor set correctly.
unsafe impl<V: VTable> VTable for CefTypePrefix<V> {
    type Kind = V::Kind;
}

impl<V: VTable, RustImpl> CefType<V, RustImpl> {

    /// construct a CefType from a vtable and a rust impl.
    /// the vtable must have all but the base parameters populated.
    /// this function will populate the base parameters.
    pub(crate) fn new(v_table: V, rust_impl: RustImpl) -> Self {
        <V::Kind as VTableKindInternal>::into_cef_type(v_table, rust_impl)
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
    fn get_base(&self) -> &<Self::Kind as VTableKindInternal>::Base {
        let self_ptr = self as *const Self;
        let base_ptr = self_ptr.cast::<<Self::Kind as VTableKindInternal>::Base>();
        unsafe { &*base_ptr }
    }

    fn get_base_mut(&mut self) -> &mut <Self::Kind as VTableKindInternal>::Base {
        let self_ptr = self as *mut Self;
        let base_ptr = self_ptr.cast::<<Self::Kind as VTableKindInternal>::Base>();
        unsafe { &mut *base_ptr }
    }

    /// # Safety
    /// this is only safe to call on types that originated from rust.
    unsafe fn get_prefix(&self) -> &CefTypePrefix<Self> {
        let self_ptr = self as *const Self;
        let prefix_ptr = self_ptr.cast::<CefTypePrefix<Self>>();
        &*prefix_ptr
    }

    /// # Safety
    /// this is only safe to call on types that originated from rust.
    unsafe fn get_prefix_mut(&mut self) -> &mut CefTypePrefix<Self> {
        let self_ptr = self as *mut Self;
        let prefix_ptr = self_ptr.cast::<CefTypePrefix<Self>>();
        &mut *prefix_ptr
    }
}

impl<V: VTable> VTableExt for V {}

/// This trait marks a type as a vtable compatible with CEF.
#[allow(private_bounds)]
pub trait VTableKind: VTableKindInternal {}

/// # Safety
///
/// This trait must only be implemented on properly constructed cef vtables.
pub(crate) unsafe trait VTableKindInternal {
    /// cef_base_ref_counted_t or cef_base_scoped_t
    type Base;

    /// CefArc<T> or CefBox<T>
    type Pointer<T: VTable<Kind = Self>>;

    /// only used for types originating from rust.
    /// AtomicUsize for CefArc, () for CefBox.
    type ExtraData;

    fn into_cef_type<V: VTable<Kind = Self>, R>(capi_v_table: V, rust_impl: R) -> CefType<V, R>;

    // /// only used for types originating from rust.
    // /// AtomicUsize::new(1) for CefArc, () for CefBox.
    // fn get_initial_extra_data() -> Self::ExtraData;

    // /// # Safety
    // /// This is only safe to call on types that originated from rust.
    // unsafe fn set_base_values<T: VTable<Kind = Self>>(value: &mut T);
}

impl<T: VTableKindInternal> VTableKind for T {}
