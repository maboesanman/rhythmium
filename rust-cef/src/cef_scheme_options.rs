use cef_sys::{
    cef_scheme_options_t, cef_scheme_options_t_CEF_SCHEME_OPTION_CORS_ENABLED,
    cef_scheme_options_t_CEF_SCHEME_OPTION_CSP_BYPASSING,
    cef_scheme_options_t_CEF_SCHEME_OPTION_DISPLAY_ISOLATED,
    cef_scheme_options_t_CEF_SCHEME_OPTION_FETCH_ENABLED,
    cef_scheme_options_t_CEF_SCHEME_OPTION_LOCAL, cef_scheme_options_t_CEF_SCHEME_OPTION_SECURE,
    cef_scheme_options_t_CEF_SCHEME_OPTION_STANDARD,
};

pub struct CefSchemeOptions {
    standard: bool,
    local: bool,
    display_isolated: bool,
    secure: bool,
    cors_enabled: bool,
    csp_bypassing: bool,
    fetch_enabled: bool,
}

impl Into<cef_scheme_options_t> for CefSchemeOptions {
    fn into(self) -> cef_scheme_options_t {
        let mut value = 0;

        if self.standard {
            value |= cef_scheme_options_t_CEF_SCHEME_OPTION_STANDARD;
        }

        if self.local {
            value |= cef_scheme_options_t_CEF_SCHEME_OPTION_LOCAL;
        }

        if self.display_isolated {
            value |= cef_scheme_options_t_CEF_SCHEME_OPTION_DISPLAY_ISOLATED;
        }

        if self.secure {
            value |= cef_scheme_options_t_CEF_SCHEME_OPTION_SECURE;
        }

        if self.cors_enabled {
            value |= cef_scheme_options_t_CEF_SCHEME_OPTION_CORS_ENABLED;
        }

        if self.csp_bypassing {
            value |= cef_scheme_options_t_CEF_SCHEME_OPTION_CSP_BYPASSING;
        }

        if self.fetch_enabled {
            value |= cef_scheme_options_t_CEF_SCHEME_OPTION_FETCH_ENABLED;
        }

        value
    }
}

impl From<cef_scheme_options_t> for CefSchemeOptions {
    fn from(value: cef_scheme_options_t) -> Self {
        let standard = value & cef_scheme_options_t_CEF_SCHEME_OPTION_STANDARD != 0;
        let local = value & cef_scheme_options_t_CEF_SCHEME_OPTION_LOCAL != 0;
        let display_isolated = value & cef_scheme_options_t_CEF_SCHEME_OPTION_DISPLAY_ISOLATED != 0;
        let secure = value & cef_scheme_options_t_CEF_SCHEME_OPTION_SECURE != 0;
        let cors_enabled = value & cef_scheme_options_t_CEF_SCHEME_OPTION_CORS_ENABLED != 0;
        let csp_bypassing = value & cef_scheme_options_t_CEF_SCHEME_OPTION_CSP_BYPASSING != 0;
        let fetch_enabled = value & cef_scheme_options_t_CEF_SCHEME_OPTION_FETCH_ENABLED != 0;

        Self {
            standard,
            local,
            display_isolated,
            secure,
            cors_enabled,
            csp_bypassing,
            fetch_enabled,
        }
    }
}
