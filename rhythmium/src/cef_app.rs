use rust_cef::{
    c_to_rust::command_line::CommandLine,
    rust_to_c::{app::{App, AppConfig}, browser_process_handler::{BrowserProcessHandlerConfig, BrowserProcessHandler}},
    structs::settings::Settings,
    util::cef_arc::CefArc,
};

pub struct RhythmiumCefApp {
    browser_process_handler: CefArc<BrowserProcessHandler>,
}

impl RhythmiumCefApp {
    pub fn new() -> CefArc<App> {
        App::new(Self {
            browser_process_handler: BrowserProcessHandler::new(RhythmiumCefBrowserProcessHandler, ()),
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
            command_line.append_switch("use-mock-keychain")
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
        ..Default::default()
    }
}

pub struct RhythmiumCefBrowserProcessHandler;

impl BrowserProcessHandlerConfig for RhythmiumCefBrowserProcessHandler {
    type BrowserProcessState = ();

    fn on_schedule_message_pump_work(&self, delay_ms: u64) {
        println!("on_schedule_message_pump_work: {}", delay_ms);
    }
}
