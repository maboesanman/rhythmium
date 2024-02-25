use cef_wrapper::cef_capi_sys::{
    cef_base_ref_counted_t, cef_browser_t, cef_paint_element_type_t, cef_rect_t,
    cef_render_handler_t, cef_screen_info_t,
};

use crate::{
    c_to_rust::browser::Browser,
    enums::paint_element_type::PaintElementType,
    structs::{geometry::Rect, screen_info::ScreenInfo},
    util::{
        cef_arc::{uninit_arc_vtable, CefArc, CefArcFromRust},
        starts_with::StartsWith,
    },
};

#[repr(transparent)]
pub struct RenderHandler(pub(crate) cef_render_handler_t);

unsafe impl StartsWith<cef_render_handler_t> for RenderHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for RenderHandler {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_render_handler_t {}

impl RenderHandler {
    pub fn new<C: RenderHandlerConfig>(config: C) -> CefArc<Self> {
        let v_table = RenderHandler(cef_render_handler_t {
            base: uninit_arc_vtable(),
            get_accessibility_handler: None,
            get_root_screen_rect: None,
            get_view_rect: Some(C::get_view_rect_raw),
            get_screen_point: Some(C::get_screen_point_raw),
            get_screen_info: Some(C::get_screen_info_raw),
            on_popup_show: None,
            on_popup_size: None,
            on_paint: Some(C::on_paint_raw),
            on_accelerated_paint: None,
            get_touch_handle_size: None,
            on_touch_handle_state_changed: None,
            start_dragging: None,
            update_drag_cursor: None,
            on_scroll_offset_changed: None,
            on_ime_composition_range_changed: None,
            on_text_selection_changed: None,
            on_virtual_keyboard_requested: None,
        });
        CefArc::new(v_table, config).type_erase()
    }
}

// these methods are all called on the ui thread, so they can take mutable references to self.
pub trait RenderHandlerConfig: Sized + Send {
    fn get_view_rect(&mut self, browser: CefArc<Browser>) -> Option<Rect>;

    fn on_paint(
        &mut self,
        _browser: CefArc<Browser>,
        _paint_element_type: PaintElementType,
        _dirty_rects: &[Rect],
        _buffer: &[u8],
        _width: usize,
        _height: usize,
    ) {
    }

    fn get_screen_info(&mut self, _browser: CefArc<Browser>) -> Option<ScreenInfo> {
        None
    }

    fn get_screen_point(
        &mut self,
        _browser: CefArc<Browser>,
        _view_x: i32,
        _view_y: i32,
    ) -> Option<(i32, i32)> {
        None
    }
}

pub(crate) trait RenderHandlerConfigExt: RenderHandlerConfig {
    unsafe extern "C" fn get_view_rect_raw(
        ptr: *mut cef_render_handler_t,
        browser: *mut cef_browser_t,
        rect: *mut cef_rect_t,
    ) {
        let rust_impl_ptr =
            CefArcFromRust::<RenderHandler, Self>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &mut *rust_impl_ptr;

        let browser = browser.cast::<Browser>();
        let browser = CefArc::from_raw(browser);

        if let Some(view_rect) = rust_impl.get_view_rect(browser) {
            *rect = view_rect.into();
        }
    }

    unsafe extern "C" fn on_paint_raw(
        ptr: *mut cef_render_handler_t,
        browser: *mut cef_browser_t,
        paint_element_type: cef_paint_element_type_t,
        dirty_rects_count: usize,
        dirty_rects_start: *const cef_rect_t,
        buffer: *const ::std::os::raw::c_void,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) {
        let rust_impl_ptr =
            CefArcFromRust::<RenderHandler, Self>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &mut *rust_impl_ptr;

        let browser = browser.cast::<Browser>();
        let browser = CefArc::from_raw(browser);

        let dirty_rects = std::slice::from_raw_parts(dirty_rects_start, dirty_rects_count);
        let dirty_rects = dirty_rects
            .iter()
            .copied()
            .map(|rect| rect.into())
            .collect::<Vec<_>>();

        let buffer = std::slice::from_raw_parts(buffer.cast::<u8>(), (width * height * 4) as usize);

        rust_impl.on_paint(
            browser,
            paint_element_type.into(),
            &dirty_rects,
            buffer,
            width as usize,
            height as usize,
        );
    }

    unsafe extern "C" fn get_screen_info_raw(
        ptr: *mut cef_render_handler_t,
        browser: *mut cef_browser_t,
        screen_info: *mut cef_screen_info_t,
    ) -> ::std::os::raw::c_int {
        let rust_impl_ptr =
            CefArcFromRust::<RenderHandler, Self>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &mut *rust_impl_ptr;

        let browser = browser.cast::<Browser>();
        let browser = CefArc::from_raw(browser);

        if let Some(new_screen_info) = rust_impl.get_screen_info(browser) {
            *screen_info = new_screen_info.into();
            1
        } else {
            0
        }
    }

    unsafe extern "C" fn get_screen_point_raw(
        ptr: *mut cef_render_handler_t,
        browser: *mut cef_browser_t,
        view_x: ::std::os::raw::c_int,
        view_y: ::std::os::raw::c_int,
        screen_x: *mut ::std::os::raw::c_int,
        screen_y: *mut ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int {
        let rust_impl_ptr =
            CefArcFromRust::<RenderHandler, Self>::get_rust_impl_from_ptr(ptr.cast());
        let rust_impl = &mut *rust_impl_ptr;

        let browser = browser.cast::<Browser>();
        let browser = CefArc::from_raw(browser);

        if let Some((new_screen_x, new_screen_y)) =
            rust_impl.get_screen_point(browser, view_x, view_y)
        {
            *screen_x = new_screen_x;
            *screen_y = new_screen_y;
            1
        } else {
            0
        }
    }
}

impl<T: RenderHandlerConfig> RenderHandlerConfigExt for T {}
