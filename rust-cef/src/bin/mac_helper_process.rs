#![feature(arbitrary_self_types)]

use rust_cef::{
    app::{App, CustomApp},
    command_line::CommandLine,
    execute_process::execute_process_with_env_args,
    scheme_options::SchemeOption,
    scheme_registrar::SchemeRegistrar,
    util::{cef_arc::CefArc, cef_box::CefBox, cef_type::CefType},
};

fn main() {
    let process_type = CommandLine::new().get_switch_value("type");

    let app = match process_type.as_deref() {
        None | Some("renderer") => Some(App::new(SubProcessApp)),
        _ => None,
    };

    let exit_code = execute_process_with_env_args(app);

    std::process::exit(exit_code)
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
