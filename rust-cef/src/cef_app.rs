use crate::{
    cef_command_line::CefCommandLine,
    cef_resource_bundle_handler::CefResourceBundleHandler,
    cef_scheme_registrar::CefSchemeRegistrar,
    util::{
        cef_arc::{new_uninit_base, CefArc, CefPtrKindArc }, cef_base::{CefBase, CefBaseRaw, CefArcBase}, cef_box::CefBox, into_rust_arg::{IntoRustArg, IntoRustArgRef, IntoCArg},
    },
};
use cef_sys::{
    cef_app_t, cef_command_line_t, cef_resource_bundle_handler_t, cef_scheme_registrar_t,
    cef_string_utf16_t,
};

// #[repr(transparent)]
// pub struct CefApp(cef_app_t);

// unsafe impl CefBase for CefApp {
//     type Kind = CefPtrKindArc;

//     type CType = cef_app_t;
// }

// unsafe impl CefBaseRaw for cef_app_t {
//     type RustType = CefApp;

//     type Kind = CefPtrKindArc;
// }

// pub trait CefAppConfig: Sized {
//     fn on_before_command_line_processing(
//         &self,
//         _process_type: &str,
//         _command_line: CefArc<CefCommandLine>,
//     ) {
//     }
//     fn on_register_custom_schemes(&self, _registrar: CefBox<CefSchemeRegistrar>) {}
//     fn get_resource_bundle_handler(&self) -> Option<CefArc<CefResourceBundleHandler>> {
//         None
//     }
//     // fn get_browser_process_handler(app: &CefApp) -> CefArc<CefBrowserProcessHandler>;
//     // fn get_render_process_handler(app: &CefApp) -> CefArc<CefRenderProcessHandler>;
// }

// trait RawCefAppConfig: CefAppConfig {
//     unsafe extern "C" fn on_before_command_line_processing_raw(
//         app: *mut cef_app_t,
//         process_type: *const cef_string_utf16_t,
//         command_line: *mut cef_command_line_t,
//     ) {
//         todo!()
//     }

//     unsafe extern "C" fn on_register_custom_schemes_raw(
//         app: *mut cef_app_t,
//         registrar: *mut cef_scheme_registrar_t,
//     ) {
//         todo!()
//     }

//     unsafe extern "C" fn get_resource_bundle_handler_raw(
//         app: *mut cef_app_t,
//     ) -> *mut cef_resource_bundle_handler_t {
//         todo!()
//     }
// }

// impl<C: CefAppConfig> RawCefAppConfig for C {}

// impl CefApp {
//     pub fn new<C: CefAppConfig>() -> CefArc<Self> {
//         let app = cef_app_t {
//             base: new_uninit_base(),
//             on_before_command_line_processing: Some(C::on_before_command_line_processing_raw),
//             on_register_custom_schemes: Some(C::on_register_custom_schemes_raw),
//             get_resource_bundle_handler: None,
//             get_browser_process_handler: None,
//             get_render_process_handler: None,
//         };

//         let app = CefApp(app);

//         CefArc::new(app)
//     }
// }

pub struct CefApp;

pub trait CustomCefApp {
    fn on_before_command_line_processing(
        self: &CefArc<Self>,
        _process_type: &str,
        _command_line: CefArc<CefCommandLine>,
    ) {}
    fn on_register_custom_schemes(self: &CefArc<Self>, _registrar: CefBox<CefSchemeRegistrar>) {}
    fn get_resource_bundle_handler(self: &CefArc<Self>) -> Option<CefArc<impl CefApp>> {
        None
    }
    fn get_browser_process_handler(self: &CefArc<Self>) -> CefArc<CefBrowserProcessHandler>;
    fn get_render_process_handler(self: &CefArc<Self>) -> CefArc<CefRenderProcessHandler>;
}

impl<T: CefApp> CefArc<T> {
    fn on_before_command_line_processing(
        &self,
        _process_type: &str,
        _command_line: CefArc<CefCommandLine>,
    ) {
        todo!()
    }
    fn on_register_custom_schemes(&self, _registrar: CefBox<CefSchemeRegistrar>) {
        todo!()
    }
    // fn get_resource_bundle_handler(&self) -> Option<CefArc<impl CefApp>> {
    //     None
    // }
        // fn get_browser_process_handler(app: &CefApp) -> CefArc<CefBrowserProcessHandler>;
        // fn get_render_process_handler(app: &CefApp) -> CefArc<CefRenderProcessHandler>;
}

