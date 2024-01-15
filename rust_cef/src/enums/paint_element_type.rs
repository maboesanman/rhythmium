use cef_wrapper::cef_capi_sys::{
    cef_paint_element_type_t, cef_paint_element_type_t_PET_POPUP, cef_paint_element_type_t_PET_VIEW,
};

#[repr(u32)]
pub enum PaintElementType {
    View = cef_paint_element_type_t_PET_VIEW,
    Popup = cef_paint_element_type_t_PET_POPUP,
}

impl From<cef_paint_element_type_t> for PaintElementType {
    fn from(value: cef_paint_element_type_t) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<PaintElementType> for cef_paint_element_type_t {
    fn from(val: PaintElementType) -> Self {
        unsafe { std::mem::transmute(val) }
    }
}
