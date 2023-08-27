use crate::{
    cef_command_line::CefCommandLine,
    cef_resource_bundle_handler::CefResourceBundleHandler,
    cef_scheme_registrar::CefSchemeRegistrar,
    util::{
        cef_arc::{new_uninit_base, CefArc, CefPtrKindArc }, cef_base::{CefBase, CefBaseRaw}, cef_box::CefBox, into_rust_arg::{IntoRustArg, IntoRustArgRef, IntoCArg},
    },
};
use cef_sys::{
    cef_app_t, cef_command_line_t, cef_resource_bundle_handler_t, cef_scheme_registrar_t,
    cef_string_utf16_t,
};

#[repr(transparent)]
pub struct CefApp(cef_app_t);

unsafe impl CefBase for CefApp {
    type Kind = CefPtrKindArc;

    type CType = cef_app_t;
}

unsafe impl CefBaseRaw for cef_app_t {
    type RustType = CefApp;

    type Kind = CefPtrKindArc;
}

pub trait CefAppConfig: Sized {
    fn on_before_command_line_processing(
        _app: &CefApp,
        _process_type: &str,
        _command_line: CefArc<CefCommandLine>,
    ) {
    }
    fn on_register_custom_schemes(_app: &CefApp, _registrar: CefBox<CefSchemeRegistrar>) {}
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
        let app = app.into_rust_arg_ref();

        // strings are be converted to rust &str.
        let process_type = process_type.into_rust_arg();
        let process_type = &process_type;

        // cef types should be passed as CefArc, so the callback decrements the ref count.
        let command_line = command_line.into_rust_arg();

        Self::on_before_command_line_processing(app, process_type, command_line);
    }

    unsafe extern "C" fn on_register_custom_schemes_raw(
        app: *mut cef_app_t,
        registrar: *mut cef_scheme_registrar_t,
    ) {
        // the first argument are passed as reference, so the callback doesn't decrement the ref count.
        let app = app.into_rust_arg_ref();

        // cef types should be passed as CefArc, so the callback decrements the ref count.
        let registrar = registrar.into_rust_arg();

        Self::on_register_custom_schemes(app, registrar);
    }

    unsafe extern "C" fn get_resource_bundle_handler_raw(
        app: *mut cef_app_t,
    ) -> *mut cef_resource_bundle_handler_t {
        // the first argument must be passed as reference.
        let app = app.into_rust_arg_ref();

        let result = Self::get_resource_bundle_handler(app);

        result.map(|handler| handler.into_c_arg())
            .unwrap_or(std::ptr::null_mut())
    }
}

impl<C: CefAppConfig> RawCefAppConfig for C {}

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
