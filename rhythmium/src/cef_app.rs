use rust_cef::{
    c_to_rust::command_line::CommandLine,
    rust_to_c::app::{App, AppConfig},
    structs::settings::Settings,
    util::cef_arc::CefArc,
};

pub struct RhythmiumCefApp;

impl RhythmiumCefApp {
    pub fn new() -> CefArc<App> {
        App::new(Self, (), ())
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
            command_line.append_switch("use-mock-keychain")
        }
    }
}

pub fn get_settings() -> Settings {
    Settings {
        windowless_rendering_enabled: true,
        ..Default::default()
    }
}
