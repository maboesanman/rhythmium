use std::process::exit;

use cef_wrapper::CefApp;
use scene::{
    image_view::{ImageFit, ImageViewBuilder},
    scene_view::SceneViewBuilder,
};
use taffy::prelude::*;
use winit::event_loop::EventLoop;

pub mod scene;

#[tokio::main]
pub async fn main() {
    env_logger::init();

    // the winit event loop needs to launch first.
    // in particular, it needs to run before the cef subprocess is launched.
    let event_loop = EventLoop::new().unwrap();

    let app = match CefApp::new() {
        Ok(app) => app.await,
        Err(e) => exit(e),
    };

    app.create_browser(|buf, w, h| {
        println!("painting {}x{} buffer", w, h);

        // convert buf from a *const c_void to a &[u8]
        let buf = unsafe { std::slice::from_raw_parts(buf.cast::<u8>(), (w * h * 4) as usize) };

        image::save_buffer("./test.png", buf, w as u32, h as u32, image::ColorType::Rgba8)
            .unwrap();
    });

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

    view_builder.add_view(
        back,
        Box::new(ImageViewBuilder::new(
            include_bytes!("../assets/bold-and-brash.jpg"),
            ImageFit::Cover,
        )),
    );

    view_builder.add_view(
        front,
        Box::new(ImageViewBuilder::new(
            include_bytes!("../assets/pointing.png"),
            ImageFit::SetWidth(scene::image_view::ImageJustification::End),
        )),
    );

    scene::view::run(event_loop, view_builder).await;
}
