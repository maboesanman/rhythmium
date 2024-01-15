use cef_wrapper::cef_capi_sys::{cef_point_t, cef_rect_t};

pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl From<cef_point_t> for Point {
    fn from(point: cef_point_t) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl Into<cef_point_t> for Point {
    fn into(self) -> cef_point_t {
        cef_point_t {
            x: self.x,
            y: self.y,
        }
    }
}

pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<cef_rect_t> for Rect {
    fn from(rect: cef_rect_t) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

impl Into<cef_rect_t> for Rect {
    fn into(self) -> cef_rect_t {
        cef_rect_t {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

pub struct Insets {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

impl From<cef_rect_t> for Insets {
    fn from(rect: cef_rect_t) -> Self {
        Self {
            top: rect.y,
            left: rect.x,
            bottom: rect.height,
            right: rect.width,
        }
    }
}

impl Into<cef_rect_t> for Insets {
    fn into(self) -> cef_rect_t {
        cef_rect_t {
            x: self.left,
            y: self.top,
            width: self.right,
            height: self.bottom,
        }
    }
}
