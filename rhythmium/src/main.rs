use scene::{
    image_view::{ImageFit, ImageViewBuilder},
    scene_view::SceneViewBuilder,
};
use taffy::prelude::*;

pub mod scene;

#[tokio::main]
pub async fn main() {
    env_logger::init();

    let mut taffy = Taffy::new();

    // let node_a = taffy
    //     .new_leaf(Style {
    //         flex_grow: 1.0,
    //         ..Default::default()
    //     })
    //     .unwrap();

    // let node_b = taffy
    //     .new_leaf(Style {
    //         flex_grow: 1.0,
    //         ..Default::default()
    //     })
    //     .unwrap();

    // let root_node = taffy
    //     .new_with_children(
    //         Style {
    //             flex_direction: FlexDirection::Row,
    //             gap: points(16.0),
    //             size: Size {
    //                 width: percent(1.0),
    //                 height: percent(1.0),
    //             },
    //             padding: Rect {
    //                 top: points(16.0),
    //                 bottom: points(16.0),
    //                 left: points(16.0),
    //                 right: points(16.0),
    //             },
    //             ..Default::default()
    //         },
    //         &[node_a, node_b],
    //     )
    //     .unwrap();

    let front = taffy
        .new_leaf(Style {
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        })
        .unwrap();

    let back = taffy
        .new_with_children(
            Style {
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                ..Default::default()
            },
            &[front],
        )
        .unwrap();

    let scene = scene::Scene {
        root: back,
        view_tree: taffy,
    };

    let mut view_builder = Box::new(SceneViewBuilder::new(scene));

    view_builder.add_view(
        back,
        Box::new(ImageViewBuilder::new(
            include_bytes!("../assets/bold-and-brash.jpg"),
            ImageFit::Contain,
        )),
    );

    view_builder.add_view(
        front,
        Box::new(ImageViewBuilder::new(
            include_bytes!("../assets/pointing.png"),
            ImageFit::SetWidth(scene::image_view::ImageJustification::End),
        )),
    );

    scene::view::run(view_builder).await;
}
