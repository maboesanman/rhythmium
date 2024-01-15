use cef_wrapper::cef_capi_sys::{cef_base_ref_counted_t, cef_life_span_handler_t};

use crate::util::starts_with::StartsWith;

#[repr(transparent)]
pub struct LifeSpanHandler(pub(crate) cef_life_span_handler_t);

unsafe impl StartsWith<cef_life_span_handler_t> for LifeSpanHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for LifeSpanHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_life_span_handler_t {}
