use crate::{
    cef_command_line::CefCommandLine,
    cef_resource_bundle_handler::CefResourceBundleHandler,
    util::{
        cef_arc::{new_uninit_base, CefArc, CefRefCounted, CefRefCountedRaw},
        cef_string::cef_string_utf16_to_rust_string,
    }, cef_scheme_registrar::CefSchemeRegistrar,
};
use cef_sys::{cef_app_t, cef_command_line_t, cef_resource_bundle_handler_t, cef_string_utf16_t, cef_scheme_registrar_t};

#[repr(transparent)]
pub struct CefApp(cef_app_t);

unsafe impl CefRefCounted for CefApp {}

unsafe impl CefRefCountedRaw for cef_app_t {
    type Wrapper = CefApp;
}

pub trait CefAppConfig: Sized {
    fn on_before_command_line_processing(
        _app: &CefApp,
        _process_type: &str,
        _command_line: CefArc<CefCommandLine>,
    ) {
    }
    fn on_register_custom_schemes(_app: &CefApp, _registrar: CefArc<CefSchemeRegistrar>) { }
    fn get_resource_bundle_handler(_app: &CefApp) -> Option<CefArc<CefResourceBundleHandler>> {
        None
    }
    // fn get_browser_process_handler(app: &CefApp) -> CefArc<CefBrowserProcessHandler>;
    // fn get_render_process_handler(app: &CefApp) -> CefArc<CefRenderProcessHandler>;
}

trait RawCefAppConfig: CefAppConfig {
    unsafe extern "C" fn on_before_command_line_processing_raw(
        app: *mut cef_app_t,
        process_type: *const cef_string_utf16_t,
        command_line: *mut cef_command_line_t,
    ) {
        // the first argument are passed as reference, so the callback doesn't decrement the ref count.
        let app = CefArc::from_ptr(app);
        let app = &*app;

        // strings are be converted to rust &str.
        let process_type = cef_string_utf16_to_rust_string(process_type);
        let process_type = &process_type;

        // cef types should be passed as CefArc, so the callback decrements the ref count.
        let command_line = CefArc::from_ptr(command_line);

        Self::on_before_command_line_processing(app, process_type, command_line);
    }

    unsafe extern "C" fn on_register_custom_schemes_raw(
        app: *mut cef_app_t,
        registrar: *mut cef_scheme_registrar_t,
    ) {
        // the first argument are passed as reference, so the callback doesn't decrement the ref count.
        let app = CefArc::from_ptr(app);
        let app = &*app;

        // cef types should be passed as CefArc, so the callback decrements the ref count.
        let registrar = CefArc::from_ptr(registrar);

        Self::on_register_custom_schemes(app, registrar);
    }

    unsafe extern "C" fn get_resource_bundle_handler_raw(
        app: *mut cef_app_t,
    ) -> *mut cef_resource_bundle_handler_t {
        // the first argument must be passed as reference.
        let app = CefArc::from_ptr(app);
        let app = &*app;

        Self::get_resource_bundle_handler(app)
            .map(|handler| handler.into_ptr())
            .unwrap_or(std::ptr::null_mut())
    }
}

impl<C: CefAppConfig> RawCefAppConfig for C { }

impl CefApp {
    pub fn new<C: CefAppConfig>() -> CefArc<Self> {
        let app = cef_app_t {
            base: new_uninit_base(),
            on_before_command_line_processing: Some(C::on_before_command_line_processing_raw),
            on_register_custom_schemes: Some(C::on_register_custom_schemes_raw),
            get_resource_bundle_handler: Some(C::get_resource_bundle_handler_raw),
            get_browser_process_handler: None,
            get_render_process_handler: None,
        };

        let app = CefApp(app);

        CefArc::new(app)
    }
}
