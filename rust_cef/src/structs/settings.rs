use cef_wrapper::cef_capi_sys::cef_settings_t;

use crate::{
    enums::{log_item::LogItems, log_severity::LogSeverity},
    util::{cef_string::str_into_cef_string_utf16, wrap_boolean::wrap_boolean},
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub accept_languages: Vec<String>,
    pub background_color: u32,
    pub browser_subprocess_path: Option<String>,
    pub cache_path: Option<String>,
    #[cfg(target_os = "windows")]
    pub chrome_app_icon_id: Option<usize>,
    pub chrome_policy_id: Option<String>,
    pub chrome_runtime: bool,
    pub command_line_args_disabled: bool,
    pub cookieable_schemes_exclude_defaults: bool,
    pub cookieable_schemes: Vec<String>,
    pub external_message_pump: bool,
    #[cfg(target_os = "macos")]
    pub framework_dir_path: Option<String>,
    pub javascript_flags: Option<String>,
    pub locale: Option<String>,
    #[cfg(not(target_os = "macos"))]
    pub locales_dir_path: Option<String>,
    pub log_file: Option<String>,
    pub log_items: LogItems,
    pub log_severity: LogSeverity,
    #[cfg(target_os = "macos")]
    pub main_bundle_path: Option<String>,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub multi_threaded_message_loop: bool,
    // no_sandbox: bool,
    pub pack_loading_disabled: bool,
    pub persist_session_cookies: bool,
    pub persist_user_preferences: bool,
    pub remote_debugging_port: Option<u16>,
    pub resources_dir_path: Option<String>,
    pub root_cache_path: Option<String>,
    pub uncaught_exception_stack_size: Option<u16>,
    pub user_agent: Option<String>,
    pub user_agent_product: Option<String>,
    pub windowless_rendering_enabled: bool,
}

impl From<&Settings> for cef_settings_t {
    fn from(value: &Settings) -> Self {
        let wrap_string =
            |s: &Option<String>| str_into_cef_string_utf16(s.as_deref().unwrap_or(""));

        cef_settings_t {
            size: std::mem::size_of::<cef_settings_t>(),
            accept_language_list: str_into_cef_string_utf16(&value.accept_languages.join(",")),
            background_color: value.background_color,
            browser_subprocess_path: wrap_string(&value.browser_subprocess_path),
            cache_path: wrap_string(&value.cache_path),
            #[cfg(target_os = "windows")]
            chrome_app_icon_id: value.chrome_app_icon_id,
            #[cfg(not(target_os = "windows"))]
            chrome_app_icon_id: 0,
            chrome_policy_id: wrap_string(&value.chrome_policy_id),
            chrome_runtime: wrap_boolean(value.chrome_runtime),
            command_line_args_disabled: wrap_boolean(value.command_line_args_disabled),
            cookieable_schemes_exclude_defaults: wrap_boolean(
                value.cookieable_schemes_exclude_defaults,
            ),
            cookieable_schemes_list: str_into_cef_string_utf16(&value.cookieable_schemes.join(",")),
            external_message_pump: wrap_boolean(value.external_message_pump),
            framework_dir_path: wrap_string(&value.framework_dir_path),
            javascript_flags: wrap_string(&value.javascript_flags),
            locale: wrap_string(&value.locale),

            #[cfg(target_os = "macos")]
            locales_dir_path: str_into_cef_string_utf16(""),
            #[cfg(not(target_os = "macos"))]
            locales_dir_path: wrap_string(&value.locales_dir_path),
            log_file: wrap_string(&value.log_file),
            log_items: value.log_items.into(),
            log_severity: value.log_severity.into(),
            main_bundle_path: wrap_string(&value.main_bundle_path),
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            multi_threaded_message_loop: wrap_boolean(value.multi_threaded_message_loop),
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            multi_threaded_message_loop: 0,
            #[cfg(sandbox)]
            no_sandbox: 0,
            #[cfg(not(sandbox))]
            no_sandbox: 1,
            pack_loading_disabled: wrap_boolean(value.pack_loading_disabled),
            persist_session_cookies: wrap_boolean(value.persist_session_cookies),
            persist_user_preferences: wrap_boolean(value.persist_user_preferences),
            remote_debugging_port: value.remote_debugging_port.unwrap_or(0) as i32,
            resources_dir_path: wrap_string(&value.resources_dir_path),
            root_cache_path: wrap_string(&value.root_cache_path),
            uncaught_exception_stack_size: value.uncaught_exception_stack_size.unwrap_or(0) as i32,
            user_agent_product: wrap_string(&value.user_agent_product),
            user_agent: wrap_string(&value.user_agent),
            windowless_rendering_enabled: wrap_boolean(value.windowless_rendering_enabled),
        }
    }
}
