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

    scene::view::run(view_builder).await;
}
