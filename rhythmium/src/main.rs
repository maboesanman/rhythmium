#![allow(clippy::new_ret_no_self)]

use std::{process::exit, thread};

use cef_app::RhythmiumCefApp;
use rust_cef::functions::initialize::initialize_from_env;
use scene::{
    image_view::{ImageFit, ImageViewBuilder},
    scene_view::SceneViewBuilder,
    view::ActiveView,
    web_view::WebViewBuilder,
};
use taffy::prelude::*;
use winit::event_loop::EventLoop;

pub mod cef_app;
pub mod scene;

pub fn main() {
    env_logger::init();

    // the winit event loop needs to launch first.
    // in particular, it needs to run before the cef subprocess is launched.
    let event_loop = EventLoop::<RhythmiumEvent>::with_user_event()
        .build()
        .unwrap();

    let proxy = event_loop.create_proxy();
    let other_proxy = proxy.clone();

    // this sends a "CatchUpOnCefWork" event every 50ms to the event loop.
    // I'm not sure what situations I'm not calling the do_work function, but
    // I'm missing something and I'm not sure what, so for now we just do a bunch of extra catch up work.
    thread::spawn(move || loop {
        other_proxy
            .send_event(RhythmiumEvent::CatchUpOnCefWork)
            .unwrap();
        thread::sleep(std::time::Duration::from_millis(50));
    });

    if let Err(e) = initialize_from_env(&cef_app::get_settings(), || RhythmiumCefApp::new(proxy)) {
        exit(e);
    }

    let mut taffy = TaffyTree::new();

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
                top: length(16.0),
                bottom: length(16.0),
                left: length(16.0),
                right: length(16.0),
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
            &[back, front],
        )
        .unwrap();

    let scene = scene::Scene {
        root,
        view_tree: taffy,
    };

    let mut view_builder = SceneViewBuilder::new(scene);

    view_builder.add_view(back, Box::new(WebViewBuilder::new()));

    view_builder.add_view(
        front,
        Box::new(ImageViewBuilder::new(
            include_bytes!("../assets/pointing.png"),
            ImageFit::SetWidth(scene::image_view::ImageJustification::End),
        )),
    );

    let mut active_view = ActiveView::new(view_builder);

    event_loop.run_app(&mut active_view).unwrap();

    // scene::view::run(event_loop, Box::new(view_builder));
}

#[derive(Debug, Clone)]
pub enum RhythmiumEvent {
    RenderFrame,
    DoCefWorkNow,
    DoCefWorkLater(u64),
    CatchUpOnCefWork,
}
