use std::collections::HashMap;

use scene::{
    image_view::{ImageFit, ImageView},
    view::{SolidColorView, View}, scene_view::SceneView,
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

    let scene = scene::Scene {
        root: root_node,
        view_tree: taffy,
    };

    

    scene::view::run(|wgpu_shared| {
        let views = {
            let mut views = HashMap::<_, Box<dyn View>>::new();
    
            views.insert(node_a, Box::new(SolidColorView::random()));
            views.insert(node_b, Box::new(ImageView::new(
                wgpu_shared.clone(),
                include_bytes!("../assets/bold-and-brash.jpg"),
                ImageFit::Cover,
            )));
    
            views
        };

        let size = wgpu_shared.window.inner_size();
        Box::new(SceneView::new(
            scene,
            views,
            size,
            wgpu_shared.clone(),
        ))
    })
    .await;
    // scene::view::run(Box::new(SolidColorView::new())).await;
}
