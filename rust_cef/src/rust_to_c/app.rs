use std::cell::UnsafeCell;

use cef_wrapper::cef_capi_sys::{
    cef_app_t, cef_base_ref_counted_t, cef_command_line_t, cef_string_t, cef_browser_process_handler_t,
};

use crate::{
    c_to_rust::{command_line::CommandLine, browser_host},
    util::{
        cef_arc::{uninit_arc_vtable, CefArc, CefArcFromRust},
        cef_string::cef_string_utf16_into_string,
        starts_with::StartsWith,
    },
};

use super::browser_process_handler::BrowserProcessHandler;

#[repr(transparent)]
pub struct App(pub(crate) cef_app_t);

unsafe impl StartsWith<cef_app_t> for App {}
unsafe impl StartsWith<cef_base_ref_counted_t> for App {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_app_t {}

impl App {
    pub fn new<C: AppConfig>(
        config: C,
        browser_process_state: C::BrowserProcessState,
        render_process_state: C::RenderProcessState,
    ) -> CefArc<Self> {
        let v_table = App(cef_app_t {
            base: uninit_arc_vtable(),
            get_browser_process_handler: Some(C::get_browser_process_handler_raw),
            get_render_process_handler: None,
            get_resource_bundle_handler: None,
            on_before_command_line_processing: Some(C::on_before_command_line_processing_raw),
            on_register_custom_schemes: None,
        });

        let browser_process_state = UnsafeCell::new(browser_process_state);
        let render_process_state = UnsafeCell::new(render_process_state);

        CefArc::new(
            v_table,
            AppWrapper {
                shared: config,
                browser_process_state,
                render_process_state,
            },
        )
        .type_erase()
    }
}

struct AppWrapper<C: AppConfig> {
    shared: C,
    browser_process_state: UnsafeCell<C::BrowserProcessState>,
    render_process_state: UnsafeCell<C::RenderProcessState>,
}

pub trait AppConfig: Sized + Send + Sync {
    type BrowserProcessState: Send;
    type RenderProcessState: Send;

    fn get_browser_process_handler(&self, browser_process_state: &Self::BrowserProcessState) -> Option<CefArc<BrowserProcessHandler>> {
        None
    }

    // fn get_render_process_handler(&self, render_process_state: &mut Self::RenderProcessState) -> Option<()> {
    //     None
    // }

    // fn get_resource_bundle_handler(&self) -> Option<()> {
    //     None
    // }

    fn on_before_command_line_processing(
        &self,
        _process_type: Option<&str>,
        _command_line: &mut CommandLine,
    ) {
    }

    // fn on_register_custom_schemes(&self, process_state: CustomSchemeProcessState<Self>, registrar: ()) {

    // }
}

pub enum CustomSchemeProcessState<'a, C: AppConfig> {
    Browser(&'a mut C::BrowserProcessState),
    Render(&'a mut C::RenderProcessState),
    Other,
}

pub(crate) trait AppConfigExt: AppConfig {

    unsafe extern "C" fn get_browser_process_handler_raw(
        ptr: *mut cef_app_t,
    ) -> *mut cef_browser_process_handler_t {
        println!("get_browser_process_handler_raw");
        let rust_impl_ptr =
            CefArcFromRust::<App, AppWrapper<Self>>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &*rust_impl_ptr;

        let browser_process_state = &*(rust_impl.browser_process_state.get() as *const _);

        let handler = rust_impl
            .shared
            .get_browser_process_handler(browser_process_state);

        match handler {
            Some(handler) => handler.into_raw().cast(),
            None => std::ptr::null_mut(),
        }
    }

    unsafe extern "C" fn on_before_command_line_processing_raw(
        ptr: *mut cef_app_t,
        process_type: *const cef_string_t,
        command_line: *mut cef_command_line_t,
    ) {
        let rust_impl_ptr =
            CefArcFromRust::<App, AppWrapper<Self>>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &*rust_impl_ptr;
        let process_type = cef_string_utf16_into_string(process_type);
        let command_line = command_line.cast::<CommandLine>();
        let command_line_mut = command_line.as_mut().unwrap();

        rust_impl
            .shared
            .on_before_command_line_processing(process_type.as_deref(), command_line_mut);

        let _ = command_line_mut;
        let _ = CefArc::from_raw(command_line);
    }
}

impl<T: AppConfig> AppConfigExt for T {}
