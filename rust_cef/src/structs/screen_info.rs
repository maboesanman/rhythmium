use cef_wrapper::cef_capi_sys::cef_screen_info_t;

use super::geometry::Rect;

///
/// Screen information used when window rendering is disabled. This structure is
/// passed as a parameter to CefRenderHandler::GetScreenInfo and should be
/// filled in by the client.
///
pub struct ScreenInfo {
    ///
    /// Device scale factor. Specifies the ratio between physical and logical
    /// pixels.
    ///
    pub device_scale_factor: f32,

    ///
    /// The screen depth in bits per pixel.
    ///
    pub depth: u32,

    ///
    /// The bits per color component. This assumes that the colors are balanced
    /// equally.
    ///
    pub depth_per_component: u32,

    ///
    /// This can be true for black and white printers.
    ///
    pub is_monochrome: bool,

    ///
    /// This is set from the rcMonitor member of MONITORINFOEX, to whit:
    ///   "A RECT structure that specifies the display monitor rectangle,
    ///   expressed in virtual-screen coordinates. Note that if the monitor
    ///   is not the primary display monitor, some of the rectangle's
    ///   coordinates may be negative values."
    //
    /// The |rect| and |available_rect| properties are used to determine the
    /// available surface for rendering popup views.
    ///
    pub rect: Rect,

    ///
    /// This is set from the rcWork member of MONITORINFOEX, to whit:
    ///   "A RECT structure that specifies the work area rectangle of the
    ///   display monitor that can be used by applications, expressed in
    ///   virtual-screen coordinates. Windows uses this rectangle to
    ///   maximize an application on the monitor. The rest of the area in
    ///   rcMonitor contains system windows such as the task bar and side
    ///   bars. Note that if the monitor is not the primary display monitor,
    ///   some of the rectangle's coordinates may be negative values".
    //
    /// The |rect| and |available_rect| properties are used to determine the
    /// available surface for rendering popup views.
    ///
    pub available_rect: Rect,
}

impl From<cef_screen_info_t> for ScreenInfo {
    fn from(info: cef_screen_info_t) -> Self {
        Self {
            device_scale_factor: info.device_scale_factor,
            depth: info.depth as u32,
            depth_per_component: info.depth_per_component as u32,
            is_monochrome: info.is_monochrome != 0,
            rect: info.rect.into(),
            available_rect: info.available_rect.into(),
        }
    }
}

impl From<ScreenInfo> for cef_screen_info_t {
    fn from(val: ScreenInfo) -> Self {
        cef_screen_info_t {
            device_scale_factor: val.device_scale_factor,
            depth: val.depth as i32,
            depth_per_component: val.depth_per_component as i32,
            is_monochrome: if val.is_monochrome { 1 } else { 0 },
            rect: val.rect.into(),
            available_rect: val.available_rect.into(),
        }
    }
}
