use parking_lot::Mutex;
use rust_cef::{
    c_to_rust::command_line::CommandLine,
    enums::log_severity::LogSeverity,
    rust_to_c::{
        app::{App, AppConfig},
        browser_process_handler::{BrowserProcessHandler, BrowserProcessHandlerConfig},
    },
    structs::settings::Settings,
    util::cef_arc::CefArc,
};
use winit::event_loop::EventLoopProxy;

use crate::RhythmiumEvent;

pub struct RhythmiumCefApp {
    browser_process_handler: CefArc<BrowserProcessHandler>,
}

impl RhythmiumCefApp {
    pub fn new(event_loop_proxy: EventLoopProxy<RhythmiumEvent>) -> CefArc<App> {
        App::new(
            Self {
                browser_process_handler: BrowserProcessHandler::new(
                    RhythmiumCefBrowserProcessHandler {
                        event_loop_proxy: Mutex::new(event_loop_proxy),
                    },
                    (),
                ),
            },
            (),
            (),
        )
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
        if process_type.is_none() {
            #[cfg(target_os = "macos")]
            command_line.append_switch("use-mock-keychain");
            command_line.append_switch_with_value("autoplay-policy", "no-user-gesture-required");
        }
    }

    fn get_browser_process_handler(
        &self,
        _browser_process_state: &Self::BrowserProcessState,
    ) -> Option<CefArc<BrowserProcessHandler>> {
        Some(self.browser_process_handler.clone())
    }
}

pub fn get_settings() -> Settings {
    #[cfg(not(feature = "bundled"))]
    let exec_dir = std::env::current_exe().unwrap();
    #[cfg(not(feature = "bundled"))]
    let parent_dir = exec_dir.parent().unwrap();

    Settings {
        windowless_rendering_enabled: true,
        external_message_pump: true,
        log_severity: LogSeverity::Debug,
        // chrome_runtime: true,
        #[cfg(not(feature = "bundled"))]
        root_cache_path: Some(
            parent_dir.join("../../cache_root"),
        ),
        #[cfg(all(target_os = "macos", not(feature = "bundled")))]
        framework_dir_path: Some(
            parent_dir.join("../../build/lib/Frameworks/Chromium Embedded Framework.framework"),
        ),
        #[cfg(all(target_os = "macos", not(feature = "bundled")))]
        main_bundle_path: Some(
            parent_dir.join("../../build/lib/rhythmium_partial_bundle.app"),
        ),
        #[cfg(all(target_os = "linux", not(feature = "bundled")))]
        resources_dir_path: Some(
            parent_dir.join("../../third_party/cef/cef_binary_121.3.2+gce31761+chromium-121.0.6167.75_linux64/Resources/"),
        ),
        ..Default::default()
    }
}

pub struct RhythmiumCefBrowserProcessHandler {
    event_loop_proxy: Mutex<EventLoopProxy<RhythmiumEvent>>,
}

impl BrowserProcessHandlerConfig for RhythmiumCefBrowserProcessHandler {
    type BrowserProcessState = ();

    fn on_schedule_message_pump_work(&self, delay_ms: u64) {
        match delay_ms {
            0 => self
                .event_loop_proxy
                .lock()
                .send_event(RhythmiumEvent::DoCefWorkNow),
            t => self
                .event_loop_proxy
                .lock()
                .send_event(RhythmiumEvent::DoCefWorkLater(t)),
        }
        .unwrap();
    }
}
