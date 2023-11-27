use cef_sys::{
    cef_scheme_options_t, cef_scheme_options_t_CEF_SCHEME_OPTION_CORS_ENABLED,
    cef_scheme_options_t_CEF_SCHEME_OPTION_CSP_BYPASSING,
    cef_scheme_options_t_CEF_SCHEME_OPTION_DISPLAY_ISOLATED,
    cef_scheme_options_t_CEF_SCHEME_OPTION_FETCH_ENABLED,
    cef_scheme_options_t_CEF_SCHEME_OPTION_LOCAL, cef_scheme_options_t_CEF_SCHEME_OPTION_NONE,
    cef_scheme_options_t_CEF_SCHEME_OPTION_SECURE, cef_scheme_options_t_CEF_SCHEME_OPTION_STANDARD,
};

use flagset::{flags, FlagSet};

flags! {
    pub enum SchemeOption: cef_scheme_options_t {
        None = cef_scheme_options_t_CEF_SCHEME_OPTION_NONE,
        Standard = cef_scheme_options_t_CEF_SCHEME_OPTION_STANDARD,
        Local = cef_scheme_options_t_CEF_SCHEME_OPTION_LOCAL,
        DisplayIsolated = cef_scheme_options_t_CEF_SCHEME_OPTION_DISPLAY_ISOLATED,
        Secure = cef_scheme_options_t_CEF_SCHEME_OPTION_SECURE,
        CorsEnabled = cef_scheme_options_t_CEF_SCHEME_OPTION_CORS_ENABLED,
        CspBypassing = cef_scheme_options_t_CEF_SCHEME_OPTION_CSP_BYPASSING,
        FetchEnabled = cef_scheme_options_t_CEF_SCHEME_OPTION_FETCH_ENABLED,
    }
}

pub type SchemeOptions = FlagSet<SchemeOption>;
