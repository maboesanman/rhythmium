use cef_wrapper::cef_capi_sys::cef_initialize;

use crate::{
    rust_to_c::app::App,
    structs::{main_args::MainArgs, settings::Settings},
    util::cef_arc::CefArc,
};

use super::try_start_subprocess::try_start_subprocess;

pub fn initialize<F>(main_args: MainArgs, settings: &Settings, app_factory: F) -> Result<(), i32>
where
    F: FnOnce() -> CefArc<App>,
{
    try_start_subprocess(&main_args);
    unsafe {
        cef_initialize(
            &main_args.into(),
            &settings.into(),
            app_factory().into_raw().cast(),
            std::ptr::null_mut(),
        );
    }
    Ok(())
}

pub fn initialize_from_env<F>(settings: &Settings, app_factory: F) -> Result<(), i32>
where
    F: FnOnce() -> CefArc<App>,
{
    let main_args = MainArgs::from_env();
    initialize(main_args, settings, app_factory)
}
