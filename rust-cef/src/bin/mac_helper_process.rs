#![feature(arbitrary_self_types)]

use rust_cef::{
    app::{App, CustomApp},
    command_line::CommandLine,
    execute_process::execute_process,
    scheme_options::SchemeOption,
    scheme_registrar::SchemeRegistrar,
    util::{cef_arc::CefArc, cef_box::CefBox, cef_type::CefType},
};

fn main() {
    let app = match CommandLine::new().get_switch_value("type").as_deref() {
        None | Some("renderer") => Some(App::new(SubProcessApp)),
        _ => None,
    };

    std::process::exit(execute_process(std::env::args().collect(), app))
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
