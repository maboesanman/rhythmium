use cef_sys::{
    cef_log_items_t, cef_log_items_t_LOG_ITEMS_DEFAULT, cef_log_items_t_LOG_ITEMS_FLAG_PROCESS_ID,
    cef_log_items_t_LOG_ITEMS_FLAG_THREAD_ID, cef_log_items_t_LOG_ITEMS_FLAG_TICK_COUNT,
    cef_log_items_t_LOG_ITEMS_FLAG_TIME_STAMP, cef_log_items_t_LOG_ITEMS_NONE,
};

use flagset::{flags, FlagSet};

flags! {
    pub enum LogItem: cef_log_items_t {
        Default = cef_log_items_t_LOG_ITEMS_DEFAULT,
        None = cef_log_items_t_LOG_ITEMS_NONE,
        ProcessId = cef_log_items_t_LOG_ITEMS_FLAG_PROCESS_ID,
        ThreadId = cef_log_items_t_LOG_ITEMS_FLAG_THREAD_ID,
        TimeStamp = cef_log_items_t_LOG_ITEMS_FLAG_TIME_STAMP,
        TickCount = cef_log_items_t_LOG_ITEMS_FLAG_TICK_COUNT,
    }
}

pub type LogItems = FlagSet<LogItem>;
