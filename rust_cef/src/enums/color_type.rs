use cef_wrapper::cef_capi_sys::{
    cef_color_type_t, cef_color_type_t_CEF_COLOR_TYPE_BGRA_8888,
    cef_color_type_t_CEF_COLOR_TYPE_NUM_VALUES, cef_color_type_t_CEF_COLOR_TYPE_RGBA_8888,
};

#[repr(u32)]
pub enum ColorType {
    Bgra8888 = cef_color_type_t_CEF_COLOR_TYPE_BGRA_8888,
    Rgba8888 = cef_color_type_t_CEF_COLOR_TYPE_RGBA_8888,
    NumValues = cef_color_type_t_CEF_COLOR_TYPE_NUM_VALUES,
}

impl From<cef_color_type_t> for ColorType {
    fn from(value: cef_color_type_t) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<ColorType> for cef_color_type_t {
    fn from(val: ColorType) -> Self {
        unsafe { std::mem::transmute(val) }
    }
}
