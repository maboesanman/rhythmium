use std::collections::HashMap;

use scene::{
    image_view::{ImageFit, ImageView},
    view::SolidColorView,
};
use taffy::{
    geometry::{Rect, Size},
    style::{FlexDirection, Style},
    style_helpers::{percent, points},
    Taffy,
};

pub mod scene;

#[tokio::main]
pub async fn main() {
    env_logger::init();

    let mut taffy = Taffy::new();

    let node_a = taffy
        .new_leaf(Style {
            flex_grow: 1.0,
            ..Default::default()
        })
        .unwrap();

    let node_b = taffy
        .new_leaf(Style {
            flex_grow: 1.0,
            ..Default::default()
        })
        .unwrap();

    let root_node = taffy
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Row,
                gap: points(16.0),
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                padding: Rect {
                    top: points(16.0),
                    bottom: points(16.0),
                    left: points(16.0),
                    right: points(16.0),
                },
                ..Default::default()
            },
            &[node_a, node_b],
        )
        .unwrap();

    let _scene = scene::Scene {
        root: root_node,
        view_tree: taffy,
        views: {
            let mut views = HashMap::new();

            views.insert(node_a, Box::new(SolidColorView::random()));
            views.insert(node_b, Box::new(SolidColorView::random()));

            views
        },
    };

    scene::view::run(|wgpu_shared| {
        let size = wgpu_shared.window.inner_size();
        Box::new(ImageView::new(
            wgpu_shared,
            size,
            include_bytes!("../assets/bold-and-brash.jpg"),
            ImageFit::Cover,
        ))
    })
    .await;
    // scene::view::run(Box::new(SolidColorView::new())).await;
}
