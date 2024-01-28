

use cef_wrapper::cef_capi_sys::cef_initialize;

use crate::{structs::{main_args::MainArgs, settings::Settings}, util::cef_arc::CefArc, rust_to_c::app::App};

pub fn initialize(
    main_args: MainArgs,
    settings: &Settings,
    app: CefArc<App>,
) {
    unsafe {
        cef_initialize(
            &main_args.into(),
            &settings.into(),
            app.into_raw().cast(),
            std::ptr::null_mut(),
        );
    }
}
