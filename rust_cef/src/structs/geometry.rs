use cef_wrapper::cef_capi_sys::{cef_point_t, cef_rect_t};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl From<Point> for cef_point_t {
    fn from(val: Point) -> Self {
        cef_point_t { x: val.x, y: val.y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl From<Rect> for cef_rect_t {
    fn from(val: Rect) -> Self {
        cef_rect_t {
            x: val.x,
            y: val.y,
            width: val.width,
            height: val.height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl From<Insets> for cef_rect_t {
    fn from(val: Insets) -> Self {
        cef_rect_t {
            x: val.left,
            y: val.top,
            width: val.right,
            height: val.bottom,
        }
    }
}
