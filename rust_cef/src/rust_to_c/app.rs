use cef_wrapper::cef_capi_sys::{cef_app_t, cef_base_ref_counted_t};

use crate::util::{cef_arc::{CefArc, uninit_arc_vtable}, starts_with::StartsWith};

#[repr(transparent)]
pub struct App(pub(crate) cef_app_t);

unsafe impl StartsWith<cef_app_t> for App {}
unsafe impl StartsWith<cef_base_ref_counted_t> for App {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_app_t {}

impl App {
    pub fn new<C: AppConfig>(config: C) -> CefArc<Self> {
        let v_table = App(cef_app_t {
            base: uninit_arc_vtable(),
            on_before_command_line_processing: None,
            on_register_custom_schemes: None,
            get_resource_bundle_handler: None,
            get_browser_process_handler: None,
            get_render_process_handler: None,
        });

        CefArc::new(v_table, config).type_erase()
    }
}

pub trait AppConfig: Sized {}

pub(crate) trait AppConfigExt: AppConfig {}

impl<T: AppConfig> AppConfigExt for T {}
