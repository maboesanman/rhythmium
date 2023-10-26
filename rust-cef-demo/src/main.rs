#![feature(ptr_metadata)]
use std::{env, mem::size_of};

use cef_sys::{
    _cef_app_t, _cef_settings_t, _cef_string_utf16_t, cef_base_ref_counted_t,
    cef_browser_host_create_browser, cef_browser_settings_t, cef_client_t,
    cef_do_message_loop_work, cef_execute_process, cef_initialize,
    cef_log_severity_t_LOGSEVERITY_WARNING, cef_main_args_t, cef_run_message_loop,
    cef_window_info_t,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::macos::WindowExtMacOS,
    window::WindowBuilder,
};

use crate::{app::initialize_cef_app, client::initialize_cef_client, strings::into_cef_str};

mod app;
mod base;
mod client;
mod life_span_handler;
mod strings;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    println!("args: {:?}", args);

    let argc = args.len() as std::ffi::c_int;
    let mut args_pointers = args
        .into_iter()
        .map(std::ffi::CString::new)
        .filter_map(Result::ok)
        .map(|arg| arg.as_ptr() as *mut std::ffi::c_char)
        .chain([std::ptr::null_mut()])
        .collect::<Vec<_>>();
    let argv = args_pointers.as_mut_ptr();

    let main_args = cef_main_args_t { argc, argv };

    let mut app = cef_sys::cef_app_t {
        base: cef_base_ref_counted_t {
            size: 0,
            add_ref: None,
            release: None,
            has_one_ref: None,
            has_at_least_one_ref: None,
        },
        on_before_command_line_processing: None,
        on_register_custom_schemes: None,
        get_resource_bundle_handler: None,
        get_browser_process_handler: None,
        get_render_process_handler: None,
    };

    initialize_cef_app(&mut app);
    println!("initialize app");

    let code = unsafe { cef_execute_process(&main_args, &mut app as *mut _, std::ptr::null_mut()) };
    println!("execute process");

    // if code >= 0 {
    //     println!("cef_execute_process failed with error code {}", code);
    //     std::process::exit(code);
    // }

    let settings = _cef_settings_t {
        size: size_of::<_cef_settings_t>(),
        no_sandbox: 0,
        browser_subprocess_path: into_cef_str(""),
        framework_dir_path: into_cef_str(""),
        main_bundle_path: into_cef_str(""),
        chrome_runtime: 0,
        multi_threaded_message_loop: 0,
        external_message_pump: 0,
        windowless_rendering_enabled: 0,
        command_line_args_disabled: 0,
        cache_path: into_cef_str(""),
        root_cache_path: into_cef_str(""),
        persist_session_cookies: 0,
        persist_user_preferences: 0,
        user_agent: into_cef_str(""),
        user_agent_product: into_cef_str(""),
        locale: into_cef_str(""),
        log_file: into_cef_str(""),
        log_severity: cef_log_severity_t_LOGSEVERITY_WARNING,
        javascript_flags: into_cef_str(""),
        resources_dir_path: into_cef_str(""),
        locales_dir_path: into_cef_str(""),
        pack_loading_disabled: 0,
        remote_debugging_port: 0,
        uncaught_exception_stack_size: 0,
        background_color: 0,
        accept_language_list: into_cef_str(""),
        cookieable_schemes_list: into_cef_str(""),
        cookieable_schemes_exclude_defaults: 0,
        chrome_policy_id: into_cef_str(""),
        log_items: 0,
    };

    let success =
        unsafe { cef_initialize(&main_args, &settings, &mut app, std::ptr::null_mut()) == 1 };
    println!("initialize cef");

    if !success {
        println!("cef_initialize failed");

        std::process::exit(code);
    }

    println!("initialize cef");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let window_info = cef_window_info_t {
        window_name: into_cef_str(""),
        bounds: cef_sys::_cef_rect_t {
            x: 0,
            y: 0,
            width: 600,
            height: 400,
        },
        hidden: 0,
        parent_view: window.ns_view(),
        windowless_rendering_enabled: 0,
        shared_texture_enabled: 0,
        external_begin_frame_enabled: 0,
        view: std::ptr::null_mut(),
    };

    let browser_settings = cef_browser_settings_t {
        size: size_of::<cef_browser_settings_t>(),
        windowless_frame_rate: 60,
        standard_font_family: into_cef_str(""),
        fixed_font_family: into_cef_str(""),
        serif_font_family: into_cef_str(""),
        sans_serif_font_family: into_cef_str(""),
        cursive_font_family: into_cef_str(""),
        fantasy_font_family: into_cef_str(""),
        default_font_size: 0,
        default_fixed_font_size: 0,
        minimum_font_size: 0,
        minimum_logical_font_size: 0,
        default_encoding: into_cef_str(""),
        remote_fonts: 0,
        javascript: 0,
        javascript_close_windows: 0,
        javascript_access_clipboard: 0,
        javascript_dom_paste: 0,
        image_loading: 0,
        image_shrink_standalone_to_fit: 0,
        text_area_resize: 0,
        tab_to_links: 0,
        local_storage: 0,
        databases: 0,
        webgl: 1,
        background_color: 0,
        chrome_status_bubble: 0,
        chrome_zoom_bubble: 0,
    };

    let url = into_cef_str("https://www.google.com");

    let mut client = cef_client_t {
        base: cef_base_ref_counted_t {
            size: 0,
            add_ref: None,
            release: None,
            has_one_ref: None,
            has_at_least_one_ref: None,
        },
        get_audio_handler: None,
        get_command_handler: None,
        get_context_menu_handler: None,
        get_dialog_handler: None,
        get_display_handler: None,
        get_download_handler: None,
        get_drag_handler: None,
        get_find_handler: None,
        get_focus_handler: None,
        get_frame_handler: None,
        get_permission_handler: None,
        get_jsdialog_handler: None,
        get_keyboard_handler: None,
        get_life_span_handler: None,
        get_load_handler: None,
        get_print_handler: None,
        get_render_handler: None,
        get_request_handler: None,
        on_process_message_received: None,
    };

    initialize_cef_client(&mut client);
    println!("initialize client");

    let code = unsafe {
        cef_browser_host_create_browser(
            &window_info,
            &mut client,
            &url,
            &browser_settings,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    println!("create browser");

    // if code >= 0 {
    //     println!("cef_browser_host_create_browser failed with error code {}", code);
    //     std::process::exit(code);
    // }

    unsafe { cef_run_message_loop() };

    // event_loop.run(move |event, _, control_flow| {
    // // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // // dispatched any events. This is ideal for games and similar applications.
    // control_flow.set_poll();

    // unsafe { cef_do_message_loop_work() };

    // match event {
    //     Event::WindowEvent {
    //         event: WindowEvent::CloseRequested,
    //         ..
    //     } => {
    //         println!("The close button was pressed; stopping");
    //         control_flow.set_exit();
    //     },
    //     Event::MainEventsCleared => {
    //         // Application update code.

    //         // Queue a RedrawRequested event.
    //         //
    //         // You only need to call this if you've determined that you need to redraw, in
    //         // applications which do not always need to. Applications that redraw continuously
    //         // can just render here instead.
    //         window.request_redraw();
    //     },
    //     Event::RedrawRequested(_) => {
    //         // Redraw the application.
    //         //
    //         // It's preferable for applications that do not render continuously to render in
    //         // this event rather than in MainEventsCleared, since rendering in here allows
    //         // the program to gracefully handle redraws requested by the OS.
    //     },
    //     _ => ()
    // }
    // });
}
