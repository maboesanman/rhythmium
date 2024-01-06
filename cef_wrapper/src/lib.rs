use core::{future::Future, panic};
use browser::Browser;
use futures::channel::oneshot::{self, Sender};
use std::{
    os::raw::{c_char, c_int, c_void}, ffi::c_float,
};

extern crate link_cplusplus;

pub(crate) mod sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings_c.rs"));
    include!(concat!(env!("OUT_DIR"), "/bindings_cpp.rs"));
}

pub mod browser;
pub mod browser_host;

pub struct CefApp;

impl CefApp {
    pub fn new() -> Result<impl Future<Output = Self>, i32> {
        let (sender, receiver) = oneshot::channel::<()>();

        let mut sender = Box::new(Some(sender));

        extern "C" fn app_ready(sender: *mut c_void) {
            let sender = sender.cast::<Option<Sender<()>>>();
            let sender = unsafe { sender.as_mut().unwrap() };
            let sender = sender.take().expect("app_ready called twice");
            sender.send(()).expect("app initialization failed");
        }

        let sender_ptr = sender.as_mut() as *mut _ as *mut c_void;
        let result = unsafe {
            let (argc, argv) = get_posix_args();
            sys::try_start_subprocess(argc, argv, Some(app_ready), sender_ptr)
        };

        if result != 0 {
            return Err(result);
        }

        Ok(async move {
            let _ = receiver.await;
            drop(sender);

            CefApp
        })
    }

    pub async fn create_browser(
        &self,
        get_view_rect: impl Fn(*mut c_int, *mut c_int),
        on_paint: impl Fn(*const c_void, c_int, c_int),
        get_scale_factor: impl Fn(*mut c_float),
        get_screen_point: impl Fn(c_int, c_int, *mut c_int, *mut c_int),
    ) -> Browser {
        let (sender, receiver) = oneshot::channel::<Browser>();
        let mut sender = Some(sender);
        let on_browser_created = move |browser: *mut c_void| {
            let browser = Browser::new(browser.cast());
            if let Err(_) = sender.take().unwrap().send(browser) {
                panic!("browser creation failed");
            }
        };
        let (get_view_rect, get_view_rect_arg) = anonymize_get_view_rect(get_view_rect);
        let (on_paint, on_paint_arg) = anonymize_on_paint(on_paint);
        let (on_browser_created, on_browser_created_arg) = anonymize_on_browser_created(on_browser_created);
        let (get_scale_factor, get_scale_factor_arg) = anonymize_get_scale_factor(get_scale_factor);
        let (get_screen_point, get_screen_point_arg) = anonymize_get_screen_point(get_screen_point);
        let client_settings = sys::ClientSettings {
            get_view_rect: Some(get_view_rect),
            get_view_rect_arg,
            on_paint: Some(on_paint),
            on_paint_arg,
            on_browser_created: Some(on_browser_created),
            on_browser_created_arg,
            get_scale_factor: Some(get_scale_factor),
            get_scale_factor_arg,
            get_screen_point: Some(get_screen_point),
            get_screen_point_arg,
        };

        unsafe {
            sys::create_browser(client_settings)
        };

        receiver.await.expect("browser creation failed")
    }
}

fn anonymize_on_paint<F: Fn(*const c_void, c_int, c_int)>(
    f: F,
) -> (
    unsafe extern "C" fn(*mut c_void, *const c_void, c_int, c_int),
    *mut c_void,
) {
    let ptr = Box::into_raw(Box::new(f));
    unsafe extern "C" fn call_thunk<F: Fn(*const c_void, c_int, c_int)>(
        data: *mut c_void,
        buf: *const c_void,
        w: c_int,
        h: c_int,
    ) {
        (*data.cast::<F>())(buf, w, h)
    }
    (call_thunk::<F>, ptr.cast())
}

fn anonymize_get_view_rect<F: Fn(*mut c_int, *mut c_int)>(
    f: F,
) -> (
    unsafe extern "C" fn(*mut c_void, *mut c_int, *mut c_int),
    *mut c_void,
) {
    let ptr = Box::into_raw(Box::new(f));
    unsafe extern "C" fn call_thunk<F: Fn(*mut c_int, *mut c_int)>(
        data: *mut c_void,
        w: *mut c_int,
        h: *mut c_int,
    ) {
        (*data.cast::<F>())(w, h)
    }
    (call_thunk::<F>, ptr.cast())
}

fn anonymize_on_browser_created<F: FnMut(*mut c_void)>(
    f: F,
) -> (
    unsafe extern "C" fn(*mut c_void, *mut c_void),
    *mut c_void,
) {
    let ptr = Box::into_raw(Box::new(f));
    unsafe extern "C" fn call_thunk<F: FnMut(*mut c_void)>(
        data: *mut c_void,
        browser: *mut c_void,
    ) {
        (*data.cast::<F>())(browser)
    }
    (call_thunk::<F>, ptr.cast())
}

fn anonymize_get_scale_factor<F: FnMut(*mut c_float)>(
    f: F,
) -> (
    unsafe extern "C" fn(*mut c_void, *mut c_float),
    *mut c_void,
) {
    let ptr = Box::into_raw(Box::new(f));
    unsafe extern "C" fn call_thunk<F: FnMut(*mut c_float)>(
        data: *mut c_void,
        scale_factor: *mut c_float,
    ) {
        (*data.cast::<F>())(scale_factor)
    }
    (call_thunk::<F>, ptr.cast())
}

fn anonymize_get_screen_point<F: Fn(c_int, c_int, *mut c_int, *mut c_int)>(
    f: F,
) -> (
    unsafe extern "C" fn(*mut c_void, c_int, c_int, *mut c_int, *mut c_int),
    *mut c_void,
) {
    let ptr = Box::into_raw(Box::new(f));
    unsafe extern "C" fn call_thunk<F: Fn(c_int, c_int, *mut c_int, *mut c_int)>(
        data: *mut c_void,
        x: c_int,
        y: c_int,
        screen_x: *mut c_int,
        screen_y: *mut c_int,
    ) {
        (*data.cast::<F>())(x, y, screen_x, screen_y)
    }
    (call_thunk::<F>, ptr.cast())
}

fn get_posix_args() -> (c_int, *mut *mut c_char) {
    // create a vector of zero terminated strings
    let args = argv::iter()
        .map(|arg| arg as *const _)
        .collect::<Box<[_]>>();

    let argc = args.len() as c_int;
    let argv = Box::into_raw(args) as *mut *mut c_char;

    (argc, argv)
}

pub fn do_cef_message_loop_work() {
    unsafe { sys::do_message_loop_work() }
}
