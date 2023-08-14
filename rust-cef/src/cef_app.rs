use cef_sys::{cef_app_t, cef_command_line_t, cef_string_utf16_t};
use crate::{util::{cef_arc::{CefRefCounted, CefArc, new_uninit_base, CefRefCountedRaw}, cef_string::cef_string_utf16_to_rust_string}, cef_command_line::CefCommandLine};

#[repr(transparent)]
pub struct CefApp(cef_app_t);

unsafe impl CefRefCounted for CefApp { }

unsafe impl CefRefCountedRaw for cef_app_t {
    type Wrapper = CefApp;
}

pub trait CefAppConfig: Sized {
    fn on_before_command_line_processing(app: &CefApp, process_type: &str, command_line: CefArc<CefCommandLine>);
    // fn on_register_custom_schemes(app: &CefApp, registrar: &CefSchemeRegistrar);
    // fn get_resource_bundle_handler(app: &CefApp) -> Option<CefResourceBundleHandler>;
    // fn get_browser_process_handler(app: &CefApp) -> Option<CefBrowserProcessHandler>;
    // fn get_render_process_handler(app: &CefApp) -> Option<CefRenderProcessHandler>;
}

trait RawCefAppConfig: CefAppConfig {
    unsafe extern "C" fn on_before_command_line_processing_raw(app: *mut cef_app_t, process_type: *const cef_string_utf16_t, command_line: *mut cef_command_line_t);
}

impl<C: CefAppConfig> RawCefAppConfig for C {
    unsafe extern "C" fn on_before_command_line_processing_raw(app: *mut cef_app_t, process_type: *const cef_string_utf16_t, command_line: *mut cef_command_line_t) {
        let app = CefArc::from_ptr(app);
        let app = &app;

        let process_type = cef_string_utf16_to_rust_string(process_type);
        let process_type = &process_type;

        let command_line = CefArc::from_ptr(command_line);

        C::on_before_command_line_processing(app, process_type, command_line);
    }
}

pub struct DefaultCefAppConfig;

impl CefAppConfig for DefaultCefAppConfig {
    fn on_before_command_line_processing(_app: &CefApp, _process_type: &str, _command_line: CefArc<CefCommandLine>) { }

    // fn on_register_custom_schemes(&self, app: &CefApp<Self>, registrar: &CefSchemeRegistrar) { }

    // fn get_resource_bundle_handler(&self, app: &CefApp<Self>) -> Option<CefResourceBundleHandler> {
    //     None
    // }

    // fn get_browser_process_handler(&self, app: &CefApp<Self>) -> Option<CefBrowserProcessHandler> {
    //     None
    // }

    // fn get_render_process_handler(&self, app: &CefApp<Self>) -> Option<CefRenderProcessHandler> {
    //     None
    // }
}

impl CefApp {
    pub fn new<C: CefAppConfig>() -> CefArc<Self> {
        let app = cef_app_t {
            base: new_uninit_base(),
            on_before_command_line_processing: Some(C::on_before_command_line_processing_raw),
            on_register_custom_schemes: None,
            get_resource_bundle_handler: None,
            get_browser_process_handler: None,
            get_render_process_handler: None,
        };

        let app = CefApp(app);

        CefArc::new(app)
    }
}
