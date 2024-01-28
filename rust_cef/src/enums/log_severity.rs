use cef_wrapper::cef_capi_sys::{cef_log_severity_t_LOGSEVERITY_DEBUG, cef_log_severity_t_LOGSEVERITY_INFO, cef_log_severity_t_LOGSEVERITY_WARNING, cef_log_severity_t_LOGSEVERITY_ERROR, cef_log_severity_t_LOGSEVERITY_FATAL, cef_log_severity_t_LOGSEVERITY_DISABLE, cef_log_severity_t};


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LogSeverity {
    Debug = cef_log_severity_t_LOGSEVERITY_DEBUG,
    #[default]
    Info = cef_log_severity_t_LOGSEVERITY_INFO,
    Warning = cef_log_severity_t_LOGSEVERITY_WARNING,
    Error = cef_log_severity_t_LOGSEVERITY_ERROR,
    Fatal = cef_log_severity_t_LOGSEVERITY_FATAL,
    Disable = cef_log_severity_t_LOGSEVERITY_DISABLE,
}

impl From<LogSeverity> for cef_log_severity_t {
    fn from(value: LogSeverity) -> Self {
        value as _
    }
}
