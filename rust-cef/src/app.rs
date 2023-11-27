use cef_sys::{cef_app_t, cef_command_line_t, cef_scheme_registrar_t, cef_string_utf16_t};

use crate::{
    command_line::CommandLine,
    scheme_registrar::SchemeRegistrar,
    util::{
        cef_arc::{new_uninit_base, CefArc, VTableKindArc},
        cef_box::CefBox,
        cef_type::{CefType, VTable},
    },
};

#[repr(transparent)]
pub struct App(cef_app_t);

unsafe impl VTable for App {
    type Kind = VTableKindArc;
}

pub trait CustomApp: Sized {
    fn on_before_command_line_processing(
        self: &CefArc<CefType<App, Self>>,
        process_type: &str,
        command_line: CefArc<CommandLine>,
    ) {
        let _ = (self, process_type, command_line);
    }

    fn on_register_custom_schemes(
        self: &CefArc<CefType<App, Self>>,
        scheme_registrar: CefBox<SchemeRegistrar>,
    ) {
        let _ = (self, scheme_registrar);
    }
    // fn get_resource_bundle_handler(self: &CefArc<CefType<CefApp, Self>>) -> Option<CefArc<impl CefApp>> {
    //     None
    // }
    // fn get_browser_process_handler(self: &CefArc<CefType<CefApp, Self>>) -> CefArc<CefBrowserProcessHandler>;
    // fn get_render_process_handler(self: &CefArc<CefType<CefApp, Self>>) -> CefArc<CefRenderProcessHandler>;
}

trait CustomAppRaw: CustomApp {
    unsafe extern "C" fn on_before_command_line_processing_raw(
        self_raw: *mut cef_app_t,
        process_type: *const cef_string_utf16_t,
        command_line: *mut cef_command_line_t,
    ) {
        let self_arc = CefArc::from_raw(self_raw.cast::<CefType<App, Self>>());
        let process_type = &crate::util::cef_string::cef_string_utf16_into_string(process_type).unwrap();
        let command_line = CefArc::from_raw(command_line.cast::<CommandLine>());

        self_arc.on_before_command_line_processing(process_type, command_line);

        self_arc.into_raw();
    }

    unsafe extern "C" fn on_register_custom_schemes_raw(
        self_raw: *mut cef_app_t,
        scheme_registrar: *mut cef_scheme_registrar_t,
    ) {
        let self_arc = CefArc::from_raw(self_raw.cast::<CefType<App, Self>>());
        let scheme_registrar = CefBox::from_raw(scheme_registrar.cast::<SchemeRegistrar>());

        self_arc.on_register_custom_schemes(scheme_registrar);

        self_arc.into_raw();
    }
}

impl<C: CustomApp> CustomAppRaw for C {}

impl App {
    pub fn new<C: CustomApp>(custom: C) -> CefArc<App> {
        let app = cef_app_t {
            base: new_uninit_base(),
            on_before_command_line_processing: Some(C::on_before_command_line_processing_raw),
            on_register_custom_schemes: Some(C::on_register_custom_schemes_raw),
            get_resource_bundle_handler: None,
            get_browser_process_handler: None,
            get_render_process_handler: None,
        };

        let cef_type = CefType::new(App(app), custom);

        CefArc::new(cef_type).type_erase()
    }
}

impl CefArc<App> {
    // fn on_before_command_line_processing(
    //     &self,
    //     process_type: &str,
    //     command_line: CefArc<CommandLine>,
    // ) {
    //     todo!()
    // }
    // fn get_resource_bundle_handler(&self) -> Option<CefArc<impl CefApp>> {
    //     None
    // }
    // fn get_browser_process_handler(app: &CefApp) -> CefArc<CefBrowserProcessHandler>;
    // fn get_render_process_handler(app: &CefApp) -> CefArc<CefRenderProcessHandler>;
}
