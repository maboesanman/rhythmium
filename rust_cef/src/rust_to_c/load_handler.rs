use cef_wrapper::cef_capi_sys::{
    cef_accelerated_paint_info_t, cef_base_ref_counted_t, cef_browser_t, cef_frame_t, cef_load_handler_t, cef_paint_element_type_t, cef_rect_t, cef_screen_info_t
};

use crate::{
    c_to_rust::browser::Browser,
    enums::{color_type::ColorType, paint_element_type::PaintElementType},
    structs::{geometry::Rect, screen_info::ScreenInfo},
    util::{
        cef_arc::{CefArc, CefArcFromRust, uninit_arc_vtable},
        starts_with::StartsWith,
    },
};

#[repr(transparent)]
pub struct LoadHandler(pub(crate) cef_load_handler_t);

unsafe impl StartsWith<cef_load_handler_t> for LoadHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for LoadHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_load_handler_t {}

impl LoadHandler {
    pub fn new<C: LoadHandlerConfig>(config: C) -> CefArc<Self> {
        let v_table = LoadHandler(cef_load_handler_t {
            base: uninit_arc_vtable(),
            on_loading_state_change: None,
            on_load_start: Some(C::on_load_end_raw),
            on_load_end: None,
            on_load_error: None,
        });
        CefArc::new(v_table, config).type_erase()
    }
}

pub use cef_wrapper::cef_capi_sys::cef_accelerated_paint_info_common_t;

// these methods are all called on the ui thread, so they can take mutable references to self.
pub trait LoadHandlerConfig: Sized + Send {
    // fn on_loading_state_change(&mut self, browser: CefArc<Browser>, is_loading: bool, can_go_back: bool, can_go_forward: bool) {}

    // fn on_load_start(&mut self, browser: CefArc<Browser>) {}

    fn on_load_end(&mut self, browser: CefArc<Browser>, status_code: u32) {
        
    }

    // fn on_load_error(&mut self, browser: CefArc<Browser>, ) {}
}

pub(crate) trait LoadHandlerConfigExt: LoadHandlerConfig {
    unsafe extern "C" fn on_load_end_raw(
        ptr: *mut cef_load_handler_t,
        browser: *mut cef_browser_t,
        _frame: *mut cef_frame_t,
        status_code: ::std::os::raw::c_uint
    ) {
        unsafe {
            let rust_impl_ptr =
                CefArcFromRust::<LoadHandler, Self>::get_rust_impl_from_ptr(ptr.cast());
            let rust_impl = &mut *rust_impl_ptr;

            let browser = browser.cast::<Browser>();
            let browser = CefArc::from_raw(browser);

            rust_impl.on_load_end(browser, status_code);
        }
    }
}

impl<T: LoadHandlerConfig> LoadHandlerConfigExt for T {}
