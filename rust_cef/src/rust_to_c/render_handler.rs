use cef_wrapper::cef_capi_sys::{cef_base_ref_counted_t, cef_render_handler_t};

use crate::util::starts_with::StartsWith;

#[repr(transparent)]
pub struct RenderHandler(pub(crate) cef_render_handler_t);

unsafe impl StartsWith<cef_render_handler_t> for RenderHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for RenderHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_render_handler_t {}
