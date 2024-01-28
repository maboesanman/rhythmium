use cef_wrapper::cef_capi_sys::{cef_app_t, cef_base_ref_counted_t};

use crate::util::{starts_with::StartsWith, cef_arc::CefArc};


#[repr(transparent)]
pub struct App(pub(crate) cef_app_t);

unsafe impl StartsWith<cef_app_t> for App {}
unsafe impl StartsWith<cef_base_ref_counted_t> for App {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_app_t {}

impl App {
    pub fn new<C: AppConfig>(config: C) -> CefArc<Self> {
        unimplemented!()
    }
}

pub trait AppConfig: Sized {

}

pub(crate) trait AppConfigExt: AppConfig {

}

impl<T: AppConfig> AppConfigExt for T {}
