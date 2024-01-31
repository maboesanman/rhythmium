use std::cell::UnsafeCell;

use cef_wrapper::cef_capi_sys::{cef_browser_process_handler_t, cef_base_ref_counted_t, _cef_browser_process_handler_t};

use crate::util::{starts_with::StartsWith, cef_arc::{CefArc, CefArcFromRust, uninit_arc_vtable}};

#[repr(transparent)]
pub struct BrowserProcessHandler(pub(crate) cef_browser_process_handler_t);

unsafe impl StartsWith<cef_browser_process_handler_t> for BrowserProcessHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for BrowserProcessHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_browser_process_handler_t {}

impl BrowserProcessHandler {
    pub fn new<C: BrowserProcessHandlerConfig>(config: C, browser_process_state: C::BrowserProcessState) -> CefArc<Self> {
        let v_table = BrowserProcessHandler(cef_browser_process_handler_t {
            base: uninit_arc_vtable(),
            on_schedule_message_pump_work: Some(C::on_schedule_message_pump_work_raw),
            on_register_custom_preferences: None,
            on_context_initialized: None,
            on_before_child_process_launch: None,
            on_already_running_app_relaunch: None,
            get_default_client: None,
        });

        CefArc::new(v_table, BrowserProcessHandlerWrapper {
            shared: config,
            browser_process_state: UnsafeCell::new(browser_process_state),
        }).type_erase()
    }
}

struct BrowserProcessHandlerWrapper<C: BrowserProcessHandlerConfig> {
    shared: C,
    browser_process_state: UnsafeCell<C::BrowserProcessState>,
}

pub trait BrowserProcessHandlerConfig: Sized + Send + Sync {
    type BrowserProcessState: Sized + Send;

    fn on_schedule_message_pump_work(&self, delay_ms: u64) {
        
    }
}

pub(crate) trait BrowserProcessHandlerConfigExt: BrowserProcessHandlerConfig {
    unsafe extern "C" fn on_schedule_message_pump_work_raw(
        ptr: *mut _cef_browser_process_handler_t,
        delay_ms: i64
    ) {
        let this = CefArcFromRust::<BrowserProcessHandler, BrowserProcessHandlerWrapper<Self>>::get_rust_impl_from_ptr(ptr.cast());
        let shared = &(*this).shared;

        shared.on_schedule_message_pump_work(delay_ms as u64);
    }
}

impl<T: BrowserProcessHandlerConfig> BrowserProcessHandlerConfigExt for T { }