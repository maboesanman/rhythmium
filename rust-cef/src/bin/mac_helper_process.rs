#![feature(arbitrary_self_types)]
// #![allow(private_in_public)]

use rust_cef::{
    app::{App, CustomApp},
    command_line::CommandLine,
    execute_process::execute_process,
    scheme_options::SchemeOption,
    scheme_registrar::SchemeRegistrar,
    util::{cef_arc::CefArc, cef_box::CefBox, cef_type::CefType},
};

fn main() {
    let main_args = std::env::args().collect::<Vec<_>>();

    println!("initialize app");

    let command_line = CommandLine::new();

    let app = match get_process_type(&command_line) {
        ProcessType::Browser | ProcessType::Renderer => Some(App::new(SubProcessApp)),
        ProcessType::Other => None,
    };

    println!("execute process");
    let exit_code = execute_process(main_args, app);

    println!("exiting code: {}", exit_code);
    std::process::exit(exit_code);
}

enum ProcessType {
    Browser,
    Renderer,
    Other,
}

fn get_process_type(command_line: &CefArc<CommandLine>) -> ProcessType {
    if command_line.has_switch("type") {
        return ProcessType::Browser;
    }

    if command_line.get_switch_value("type").as_deref() == Some("renderer") {
        return ProcessType::Renderer;
    }

    ProcessType::Other
}

pub struct SubProcessApp;

impl CustomApp for SubProcessApp {
    fn on_register_custom_schemes(
        self: &CefArc<CefType<App, Self>>,
        scheme_registrar: CefBox<SchemeRegistrar>,
    ) {
        scheme_registrar
            .add_custom_scheme(
                "client",
                SchemeOption::Standard
                    | SchemeOption::Secure
                    | SchemeOption::CorsEnabled
                    | SchemeOption::FetchEnabled,
            )
            .unwrap();
    }
}
