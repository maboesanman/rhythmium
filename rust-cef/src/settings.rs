use std::{
    os::raw::c_int,
    path::{Path, PathBuf},
};

use cef_sys::cef_settings_t;

use crate::{
    color::Color,
    log_items::LogItems,
    log_severity::LogSeverity,
    util::{
        cef_string::{path_into_cef_string_utf16, str_into_cef_string_utf16},
        wrap_boolean::wrap_boolean,
    },
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

fn string_vec_into_cef_str(s: &Vec<String>) -> cef_sys::cef_string_t {
    let s = s.join(",");
    str_into_cef_string_utf16(s.as_str())
}

impl Settings {
    pub fn get_cef_settings(&self) -> cef_settings_t {
        cef_settings_t {
            size: core::mem::size_of::<cef_settings_t>(),
            no_sandbox: wrap_boolean(self.no_sandbox),
            browser_subprocess_path: path_into_cef_str(self.browser_subprocess_path.as_deref()),
            framework_dir_path: path_into_cef_str(self.framework_dir_path.as_deref()),
            main_bundle_path: path_into_cef_str(self.main_bundle_path.as_deref()),
            chrome_runtime: wrap_boolean(self.chrome_runtime),
            multi_threaded_message_loop: wrap_boolean(self.multi_threaded_message_loop),
            external_message_pump: wrap_boolean(self.external_message_pump),
            windowless_rendering_enabled: wrap_boolean(self.windowless_rendering_enabled),
            command_line_args_disabled: wrap_boolean(self.command_line_args_disabled),
            cache_path: path_into_cef_str(self.cache_path.as_deref()),
            root_cache_path: path_into_cef_str(self.root_cache_path.as_deref()),
            persist_session_cookies: wrap_boolean(self.persist_session_cookies),
            persist_user_preferences: wrap_boolean(self.persist_user_preferences),
            user_agent: string_into_cef_str(self.user_agent.as_deref()),
            user_agent_product: string_into_cef_str(self.user_agent_product.as_deref()),
            locale: string_into_cef_str(self.locale.as_deref()),
            log_file: path_into_cef_str(self.log_file.as_deref()),
            log_severity: self.log_severity as _,
            log_items: self.log_items.bits(),
            javascript_flags: string_into_cef_str(self.javascript_flags.as_deref()),
            resources_dir_path: path_into_cef_str(self.resources_dir_path.as_deref()),
            locales_dir_path: path_into_cef_str(self.locales_dir_path.as_deref()),
            pack_loading_disabled: wrap_boolean(self.pack_loading_disabled),
            remote_debugging_port: self.remote_debugging_port.unwrap_or(0) as c_int,
            uncaught_exception_stack_size: wrap_boolean(self.uncaught_exception_stack_size),
            background_color: self.background_color,
            accept_language_list: string_vec_into_cef_str(&self.accept_language_list),
            cookieable_schemes_list: string_vec_into_cef_str(&self.cookieable_schemes_list),
            cookieable_schemes_exclude_defaults: wrap_boolean(
                self.cookieable_schemes_exclude_defaults,
            ),
            chrome_policy_id: string_into_cef_str(self.chrome_policy_id.as_deref()),
        }
    }
}
