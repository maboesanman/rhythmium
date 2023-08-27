use cef_sys::cef_command_line_t;

use crate::util::cef_arc::{CefRefCounted, CefRefCountedRaw};

#[repr(transparent)]
pub struct CefCommandLine(cef_command_line_t);

unsafe impl CefRefCounted for CefCommandLine {}

unsafe impl CefRefCountedRaw for cef_command_line_t {
    type Wrapper = CefCommandLine;
}
