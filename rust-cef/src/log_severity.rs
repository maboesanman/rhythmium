use cef_sys::{
    cef_log_severity_t, cef_log_severity_t_LOGSEVERITY_DEBUG,
    cef_log_severity_t_LOGSEVERITY_DEFAULT, cef_log_severity_t_LOGSEVERITY_DISABLE,
    cef_log_severity_t_LOGSEVERITY_ERROR, cef_log_severity_t_LOGSEVERITY_FATAL,
    cef_log_severity_t_LOGSEVERITY_INFO, cef_log_severity_t_LOGSEVERITY_WARNING,
};

fn _assert_c_uint_equals_u32(v: cef_log_severity_t) -> u32 {
    v
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum LogSeverity {
    Default = cef_log_severity_t_LOGSEVERITY_DEFAULT,
    Debug = cef_log_severity_t_LOGSEVERITY_DEBUG,
    Info = cef_log_severity_t_LOGSEVERITY_INFO,
    Warning = cef_log_severity_t_LOGSEVERITY_WARNING,
    Error = cef_log_severity_t_LOGSEVERITY_ERROR,
    Fatal = cef_log_severity_t_LOGSEVERITY_FATAL,
    Disable = cef_log_severity_t_LOGSEVERITY_DISABLE,
}

impl Default for LogSeverity {
    fn default() -> Self {
        Self::Default
    }
}
