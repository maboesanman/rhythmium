use cef_wrapper::cef_capi_sys::{cef_base_ref_counted_t, cef_browser_t, cef_life_span_handler_t};

use crate::{
    c_to_rust::browser::Browser,
    util::{
        cef_arc::{uninit_arc_vtable, CefArc, CefArcFromRust},
        starts_with::StartsWith,
    },
};

#[repr(transparent)]
pub struct LifeSpanHandler(pub(crate) cef_life_span_handler_t);

unsafe impl StartsWith<cef_life_span_handler_t> for LifeSpanHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for LifeSpanHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_life_span_handler_t {}

impl LifeSpanHandler {
    pub fn new<C: LifeSpanHandlerConfig>(config: C) -> CefArc<Self> {
        let v_table = LifeSpanHandler(cef_life_span_handler_t {
            base: uninit_arc_vtable(),
            on_before_popup: None,
            on_before_dev_tools_popup: None,
            on_after_created: Some(C::on_after_created_raw),
            do_close: None,
            on_before_close: None,
        });
        CefArc::new(v_table, config).type_erase()
    }
}

pub trait LifeSpanHandlerConfig: Sized {
    fn on_after_created(&mut self, _browser: CefArc<Browser>) {}
}

pub(crate) trait LifeSpanHandlerConfigExt: LifeSpanHandlerConfig {
    unsafe extern "C" fn on_after_created_raw(
        ptr: *mut cef_life_span_handler_t,
        browser: *mut cef_browser_t,
    ) {
        let rust_impl_ptr =
            CefArcFromRust::<LifeSpanHandler, Self>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &mut *rust_impl_ptr;

        let browser = browser.cast::<Browser>();
        let browser = CefArc::from_raw(browser);

        rust_impl.on_after_created(browser);
    }
}

impl<T: LifeSpanHandlerConfig> LifeSpanHandlerConfigExt for T {}
