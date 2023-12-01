#![feature(ptr_metadata)]

use rust_cef::{
    app::App, browser_host_create_browser::browser_host_create_browser,
    browser_settings::BrowserSettings, client::Client, initialize::initialize,
    message_loop::run_message_loop, rect::Rect, settings::Settings, window_info::WindowInfo,
};

use winit::{event_loop::EventLoop, platform::macos::WindowExtMacOS, window::WindowBuilder};

struct MyApp;
impl rust_cef::app::CustomApp for MyApp {}

struct MyClient;
impl rust_cef::client::CustomClient for MyClient {}

fn main() {
    let app = App::new(MyApp);
    let settings = Settings::default();

    let success = initialize(
        std::env::args().collect::<Vec<_>>(),
        &settings,
        Some(app.clone()),
    );

    if !success {
        println!("cef_initialize failed");

        std::process::exit(1);
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let window_info = WindowInfo {
        window_name: "rust-cef".to_string(),
        bounds: Rect {
            x: 0,
            y: 0,
            width: 600,
            height: 400,
        },
        hidden: false,
        parent_view: window.ns_view(),
        windowless_rendering_enabled: false,
        external_begin_frame_enabled: false,
        view: std::ptr::null_mut(),
    };

    let browser_settings = BrowserSettings::default();

    let url = "https://www.google.com";

    let client = Client::new(MyClient);

    let result = browser_host_create_browser(&window_info, client, url, &browser_settings);

    if let Err(code) = result {
        println!(
            "cef_browser_host_create_browser failed with error code {}",
            code
        );
        std::process::exit(code);
    }

    run_message_loop();
}
