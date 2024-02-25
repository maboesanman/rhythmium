use std::{process::exit, thread};

use cef_app::RhythmiumCefApp;
use rust_cef::functions::initialize::initialize_from_env;
use scene::{
    image_view::{ImageFit, ImageViewBuilder},
    scene_view::SceneViewBuilder,
    web_view::WebViewBuilder,
};
use taffy::prelude::*;
use winit::event_loop::{EventLoop, EventLoopBuilder};

pub mod cef_app;
pub mod scene;

pub fn main() {
    env_logger::init();

    // the winit event loop needs to launch first.
    // in particular, it needs to run before the cef subprocess is launched.
    let event_loop = EventLoopBuilder::<RhythmiumEvent>::with_user_event().build().unwrap();

    let proxy = event_loop.create_proxy();
    let other_proxy = proxy.clone();

    thread::spawn(move || {
        loop {
            other_proxy.send_event(RhythmiumEvent::CatchUpOnCefWork).unwrap();
            thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    if let Err(e) = cef_wrapper::init() {
        exit(e);
    }

    initialize_from_env(&cef_app::get_settings(), RhythmiumCefApp::new(proxy));

    let mut taffy = Taffy::new();

    let front = taffy
        .new_leaf(Style {
            grid_row: line(1),
            grid_column: line(1),
            ..Default::default()
        })
        .unwrap();

    let back = taffy
        .new_leaf(Style {
            grid_row: line(1),
            grid_column: line(1),
            margin: Rect {
                top: points(16.0),
                bottom: points(16.0),
                left: points(16.0),
                right: points(16.0),
            },
            ..Default::default()
        })
        .unwrap();

    let root = taffy
        .new_with_children(
            Style {
                display: Display::Grid,
                grid_template_columns: vec![fr(1.0)],
                grid_template_rows: vec![fr(1.0)],
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                ..Default::default()
            },
            &[front, back],
        )
        .unwrap();

    let scene = scene::Scene {
        root,
        view_tree: taffy,
    };

    let mut view_builder = Box::new(SceneViewBuilder::new(scene));

    // view_builder.add_view(back, Box::new(WebViewBuilder::new(app)));

    view_builder.add_view(
        front,
        Box::new(ImageViewBuilder::new(
            include_bytes!("../assets/pointing.png"),
            ImageFit::SetWidth(scene::image_view::ImageJustification::End),
        )),
    );

    let _image_view_builder = ImageViewBuilder::new(
        include_bytes!("../assets/pointing.png"),
        ImageFit::SetWidth(scene::image_view::ImageJustification::End),
    );

    scene::view::run(event_loop, Box::new(WebViewBuilder::new()));
    // scene::view::run(event_loop, Box::new(image_view_builder)).await;
}

#[derive(Debug, Clone)]
pub enum RhythmiumEvent {
    RenderFrame,
    DoCefWorkNow,
    DoCefWorkLater(u64),
    CatchUpOnCefWork,
}
