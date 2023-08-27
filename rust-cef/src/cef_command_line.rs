use cef_sys::cef_command_line_t;

use crate::util::{cef_arc::CefPtrKindArc, cef_base::{CefBase, CefBaseRaw}};

#[repr(transparent)]
pub struct CefCommandLine(cef_command_line_t);

unsafe impl CefBase for CefCommandLine {
    type CType = cef_command_line_t;
    type Kind = CefPtrKindArc;
}

unsafe impl CefBaseRaw for cef_command_line_t {
    type RustType = CefCommandLine;
    type Kind = CefPtrKindArc;
}
