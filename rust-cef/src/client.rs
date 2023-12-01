use cef_sys::cef_client_t;

use crate::util::{
    cef_arc::{new_uninit_base, CefArc, VTableKindArc},
    cef_type::{CefType, VTable},
};

#[repr(transparent)]
pub struct Client(cef_client_t);

unsafe impl VTable for Client {
    type Kind = VTableKindArc;
}

pub trait CustomClient: Sized {}

trait CustomClientRaw: CustomClient {}

impl<C: CustomClient> CustomClientRaw for C {}

impl Client {
    pub fn new<C: CustomClient>(custom: C) -> CefArc<Client> {
        let client = cef_client_t {
            base: new_uninit_base(),
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
            get_life_span_handler: None,
            get_load_handler: None,
            get_print_handler: None,
            get_render_handler: None,
            get_request_handler: None,
            on_process_message_received: None,
        };

        let cef_type = CefType::new(Client(client), custom);

        CefArc::new(cef_type).type_erase()
    }
}
