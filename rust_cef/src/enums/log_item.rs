

use cef_wrapper::cef_capi_sys::{cef_log_items_t, cef_log_items_t_LOG_ITEMS_FLAG_PROCESS_ID, cef_log_items_t_LOG_ITEMS_FLAG_THREAD_ID, cef_log_items_t_LOG_ITEMS_FLAG_TIME_STAMP, cef_log_items_t_LOG_ITEMS_FLAG_TICK_COUNT, cef_log_items_t_LOG_ITEMS_DEFAULT, cef_log_items_t_LOG_ITEMS_NONE};
use flagset::{FlagSet, flags};

flags! {
    pub enum LogItem: cef_log_items_t {
        None = cef_log_items_t_LOG_ITEMS_NONE,
        ProcessId = cef_log_items_t_LOG_ITEMS_FLAG_PROCESS_ID,
        ThreadId = cef_log_items_t_LOG_ITEMS_FLAG_THREAD_ID,
        TimeStamp = cef_log_items_t_LOG_ITEMS_FLAG_TIME_STAMP,
        TickCount = cef_log_items_t_LOG_ITEMS_FLAG_TICK_COUNT,
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LogItems(FlagSet<LogItem>);

impl Default for LogItems {
    fn default() -> Self {
        let default_flags: FlagSet<LogItem> = unsafe { core::mem::transmute(cef_log_items_t_LOG_ITEMS_DEFAULT) };
        Self(default_flags)
    }
}

impl LogItems {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        let empty_flags: FlagSet<LogItem> = unsafe { core::mem::transmute(cef_log_items_t_LOG_ITEMS_NONE) };
        Self(empty_flags)
    }
}

impl From<LogItems> for cef_log_items_t {
    fn from(value: LogItems) -> Self {
        value.0.bits()
    }
}
