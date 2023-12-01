#![feature(cfg_sanitize)]
#![feature(arbitrary_self_types)]
#![feature(ptr_metadata)]

#[macro_use]
pub mod macros;

pub mod util;

pub use cef_sys;

// pub mod resource_bundle_handler;
pub mod app;
pub mod browser_host_create_browser;
pub mod browser_settings;
pub mod client;
pub mod color;
pub mod command_line;
pub mod execute_process;
pub mod initialize;
pub mod log_items;
pub mod log_severity;
pub mod rect;
pub mod scheme_options;
pub mod scheme_registrar;
pub mod settings;
pub mod window_info;
pub mod message_loop;