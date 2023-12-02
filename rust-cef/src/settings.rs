use std::{
    os::raw::c_int,
    path::{Path, PathBuf},
};

use cef_sys::cef_settings_t;

use crate::{
    color::Color,
    log_items::LogItems,
    log_severity::LogSeverity,
    util::cef_string::{path_into_cef_string_utf16, str_into_cef_string_utf16},
};

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub no_sandbox: bool,
    pub browser_subprocess_path: Option<PathBuf>,
    pub framework_dir_path: Option<PathBuf>,
    pub main_bundle_path: Option<PathBuf>,
    pub chrome_runtime: bool,
    pub multi_threaded_message_loop: bool,
    pub external_message_pump: bool,
    pub windowless_rendering_enabled: bool,
    pub command_line_args_disabled: bool,
    pub cache_path: Option<PathBuf>,
    pub root_cache_path: Option<PathBuf>,
    pub persist_session_cookies: bool,
    pub persist_user_preferences: bool,
    pub user_agent: Option<String>,
    pub user_agent_product: Option<String>,
    pub locale: Option<String>,
    pub log_file: Option<PathBuf>,
    pub log_severity: LogSeverity,
    pub log_items: LogItems,
    pub javascript_flags: Option<String>,
    pub resources_dir_path: Option<PathBuf>,
    pub locales_dir_path: Option<PathBuf>,
    pub pack_loading_disabled: bool,
    pub remote_debugging_port: Option<u16>,
    pub uncaught_exception_stack_size: bool,
    pub background_color: Color,
    pub accept_language_list: Vec<String>,
    pub cookieable_schemes_list: Vec<String>,
    pub cookieable_schemes_exclude_defaults: bool,
    pub chrome_policy_id: Option<String>,
}

fn path_into_cef_str(p: Option<&Path>) -> cef_sys::cef_string_t {
    let p = p.unwrap_or_else(|| Path::new(""));
    path_into_cef_string_utf16(p)
}

fn string_into_cef_str(s: Option<&str>) -> cef_sys::cef_string_t {
    let s = s.unwrap_or("");
    str_into_cef_string_utf16(s)
}

fn string_vec_into_cef_str(s: &[String]) -> cef_sys::cef_string_t {
    let s = s.join(",");
    str_into_cef_string_utf16(s.as_str())
}

impl Settings {
    #[must_use]
    pub fn get_cef_settings(&self) -> cef_settings_t {
        cef_settings_t {
            size: core::mem::size_of::<cef_settings_t>(),
            no_sandbox: self.no_sandbox.into(),
            browser_subprocess_path: path_into_cef_str(self.browser_subprocess_path.as_deref()),
            framework_dir_path: path_into_cef_str(self.framework_dir_path.as_deref()),
            main_bundle_path: path_into_cef_str(self.main_bundle_path.as_deref()),
            chrome_runtime: self.chrome_runtime.into(),
            multi_threaded_message_loop: self.multi_threaded_message_loop.into(),
            external_message_pump: self.external_message_pump.into(),
            windowless_rendering_enabled: self.windowless_rendering_enabled.into(),
            command_line_args_disabled: self.command_line_args_disabled.into(),
            cache_path: path_into_cef_str(self.cache_path.as_deref()),
            root_cache_path: path_into_cef_str(self.root_cache_path.as_deref()),
            persist_session_cookies: self.persist_session_cookies.into(),
            persist_user_preferences: self.persist_user_preferences.into(),
            user_agent: string_into_cef_str(self.user_agent.as_deref()),
            user_agent_product: string_into_cef_str(self.user_agent_product.as_deref()),
            locale: string_into_cef_str(self.locale.as_deref()),
            log_file: path_into_cef_str(self.log_file.as_deref()),
            log_severity: self.log_severity as _,
            log_items: self.log_items.bits(),
            javascript_flags: string_into_cef_str(self.javascript_flags.as_deref()),
            resources_dir_path: path_into_cef_str(self.resources_dir_path.as_deref()),
            locales_dir_path: path_into_cef_str(self.locales_dir_path.as_deref()),
            pack_loading_disabled: self.pack_loading_disabled.into(),
            remote_debugging_port: c_int::from(self.remote_debugging_port.unwrap_or(0)),
            uncaught_exception_stack_size: self.uncaught_exception_stack_size.into(),
            background_color: self.background_color,
            accept_language_list: string_vec_into_cef_str(&self.accept_language_list),
            cookieable_schemes_list: string_vec_into_cef_str(&self.cookieable_schemes_list),
            cookieable_schemes_exclude_defaults: self.cookieable_schemes_exclude_defaults.into(),
            chrome_policy_id: string_into_cef_str(self.chrome_policy_id.as_deref()),
        }
    }
}
