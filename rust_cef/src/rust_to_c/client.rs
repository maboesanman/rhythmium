use crate::util::{cef_arc::{CefArc, VTableKindArc}, cef_type::VTable};




pub trait Client {
    type LifespanHandler: VTable<Kind = VTableKindArc>;

    fn get_lifespan_handler(&self) -> Option<CefArc<Self::LifespanHandler>> {
        None
    }
}

pub(crate) trait ClientExt {

}