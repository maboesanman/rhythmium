// use crate::util::{cef_arc::{CefArc, VTableKindArc}, cef_type::VTable};

use cef_wrapper::cef_capi_sys::{
    cef_base_ref_counted_t, cef_client_t, cef_life_span_handler_t, cef_render_handler_t,
};

use crate::util::{
    cef_arc::{uninit_arc_vtable, CefArc, CefArcFromRust},
    starts_with::StartsWith,
};

use super::{life_span_handler::LifeSpanHandler, render_handler::RenderHandler};

#[repr(transparent)]
pub struct Client(pub(crate) cef_client_t);

unsafe impl StartsWith<cef_client_t> for Client {}
unsafe impl StartsWith<cef_base_ref_counted_t> for Client {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_client_t {}

impl Client {
    pub fn new<C: ClientConfig>(config: C) -> CefArc<Self> {
        let v_table = Client(cef_client_t {
            base: uninit_arc_vtable(),
            get_audio_handler: None,
            get_command_handler: None,
            get_context_menu_handler: None,
            get_dialog_handler: None,
            get_display_handler: None,
            get_download_handler: None,
            get_drag_handler: None,
            get_find_handler: None,
            get_focus_handler: None,
            get_frame_handler: None,
            get_permission_handler: None,
            get_jsdialog_handler: None,
            get_keyboard_handler: None,
            get_life_span_handler: Some(C::get_life_span_handler_raw),
            get_load_handler: None,
            get_print_handler: None,
            get_render_handler: Some(C::get_render_handler_raw),
            get_request_handler: None,
            on_process_message_received: None,
        });
        CefArc::new(v_table, config).type_erase()
    }
}

pub trait ClientConfig: Sized + Send + Sync {
    fn get_life_span_handler(&self) -> Option<CefArc<LifeSpanHandler>> {
        None
    }

    fn get_render_handler(&self) -> Option<CefArc<RenderHandler>> {
        None
    }
}

pub(crate) trait ClientConfigExt: ClientConfig {
    unsafe extern "C" fn get_life_span_handler_raw(
        ptr: *mut cef_client_t,
    ) -> *mut cef_life_span_handler_t {
        let rust_impl_ptr = CefArcFromRust::<Client, Self>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &mut *rust_impl_ptr;
        let life_span_handler = rust_impl.get_life_span_handler();

        match life_span_handler {
            Some(life_span_handler) => life_span_handler
                .type_erase::<cef_life_span_handler_t>()
                .into_raw(),
            None => std::ptr::null_mut(),
        }
    }

    unsafe extern "C" fn get_render_handler_raw(
        ptr: *mut cef_client_t,
    ) -> *mut cef_render_handler_t {
        let rust_impl_ptr = CefArcFromRust::<Client, Self>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &mut *rust_impl_ptr;
        let render_handler = rust_impl.get_render_handler();

        match render_handler {
            Some(render_handler) => render_handler
                .type_erase::<cef_render_handler_t>()
                .into_raw(),
            None => std::ptr::null_mut(),
        }
    }
}

impl<T: ClientConfig> ClientConfigExt for T {}
