use parking_lot::Mutex;
use rust_cef::{
    c_to_rust::command_line::CommandLine,
    rust_to_c::{app::{App, AppConfig}, browser_process_handler::{BrowserProcessHandlerConfig, BrowserProcessHandler}},
    structs::settings::Settings,
    util::cef_arc::CefArc, enums::log_severity::LogSeverity,
};
use winit::event_loop::EventLoopProxy;

use crate::RhythmiumEvent;

pub struct RhythmiumCefApp {
    browser_process_handler: CefArc<BrowserProcessHandler>,
}

impl RhythmiumCefApp {
    pub fn new(event_loop_proxy: EventLoopProxy<RhythmiumEvent>) -> CefArc<App> {
        App::new(Self {
            browser_process_handler: BrowserProcessHandler::new(RhythmiumCefBrowserProcessHandler {
                event_loop_proxy: Mutex::new(event_loop_proxy),
            }, ()),
        }, (), ())
    }
}

impl AppConfig for RhythmiumCefApp {
    type BrowserProcessState = ();
    type RenderProcessState = ();

    fn on_before_command_line_processing(
        &self,
        process_type: Option<&str>,
        command_line: &mut CommandLine,
    ) {
        println!("on_before_command_line_processing");
        if process_type.is_none() {
            command_line.append_switch("use-mock-keychain");
            command_line.append_switch_with_value("autoplay-policy", "no-user-gesture-required");
        }
    }
    
    fn get_browser_process_handler(&self, _browser_process_state: &Self::BrowserProcessState) -> Option<CefArc<BrowserProcessHandler>> {
        println!("get_browser_process_handler");
        Some(self.browser_process_handler.clone())
    }
}

pub fn get_settings() -> Settings {
    Settings {
        windowless_rendering_enabled: true,
        external_message_pump: true,
        log_severity: LogSeverity::Debug,
        root_cache_path: Some("/Users/mason/Source/github.com/maboesanman/rhythmium/cache_root".to_string()),
        ..Default::default()
    }
}

pub struct RhythmiumCefBrowserProcessHandler {
    event_loop_proxy: Mutex<EventLoopProxy<RhythmiumEvent>>
}

impl BrowserProcessHandlerConfig for RhythmiumCefBrowserProcessHandler {
    type BrowserProcessState = ();

    fn on_schedule_message_pump_work(&self, delay_ms: u64) {
        match delay_ms {
            0 => self.event_loop_proxy.lock().send_event(RhythmiumEvent::DoCefWorkNow),
            t => self.event_loop_proxy.lock().send_event(RhythmiumEvent::DoCefWorkLater(t)),
        }.unwrap();
    }
}
