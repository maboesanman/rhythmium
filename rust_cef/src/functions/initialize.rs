use cef_wrapper::cef_capi_sys::cef_initialize;

use crate::{
    rust_to_c::app::App,
    structs::{main_args::MainArgs, settings::Settings},
    util::cef_arc::CefArc,
};

pub fn initialize(main_args: MainArgs, settings: &Settings, app: CefArc<App>) {
    unsafe {
        cef_initialize(
            &main_args.into(),
            &settings.into(),
            app.into_raw().cast(),
            std::ptr::null_mut(),
        );
    }
}

pub fn initialize_from_env(settings: &Settings, app: CefArc<App>) {
    let main_args = MainArgs::from_env();
    initialize(main_args, settings, app)
}
