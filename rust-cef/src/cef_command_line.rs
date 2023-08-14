use cef_sys::cef_command_line_t;

use crate::util::cef_arc::{CefRefCounted, CefRefCountedRaw};

#[repr(transparent)]
pub struct CefCommandLine(cef_command_line_t);

unsafe impl CefRefCounted for CefCommandLine { }

unsafe impl CefRefCountedRaw for cef_command_line_t {
    type Wrapper = CefCommandLine;
}

impl From<*mut cef_command_line_t> for CefCommandLine {
    fn from(ptr: *mut cef_command_line_t) -> Self {
        Self(unsafe { *ptr })
    }
}