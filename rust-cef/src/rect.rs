use cef_sys::cef_rect_t;

#[derive(Debug, Clone, Default)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn get_cef_rect(&self) -> cef_rect_t {
        cef_rect_t {
            x: self.x as i32,
            y: self.y as i32,
            width: self.width as i32,
            height: self.height as i32,
        }
    }
}
