use std::collections::HashMap;

use taffy::{
    geometry::{Rect, Size},
    style::{FlexDirection, Style},
    style_helpers::{percent, points},
    Taffy,
};
use view::{View, DummyView};

pub mod scene;
pub mod view;

pub fn main() {
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
                flex_direction: FlexDirection::Column,
                gap: points(11.0),
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                padding: Rect {
                    top: points(10.0),
                    bottom: points(10.0),
                    left: points(10.0),
                    right: points(10.0),
                },
                ..Default::default()
            },
            &[node_a, node_b],
        )
        .unwrap();

    let root_again = taffy
        .new_with_children(
            Style {
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                padding: Rect {
                    top: points(10.0),
                    bottom: points(10.0),
                    left: points(10.0),
                    right: points(10.0),
                },
                ..Default::default()
            },
            &[root_node],
        )
        .unwrap();

    let root_another_time = taffy
        .new_with_children(
            Style {
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                padding: Rect {
                    top: points(10.0),
                    bottom: points(10.0),
                    left: points(10.0),
                    right: points(10.0),
                },
                ..Default::default()
            },
            &[root_again],
        )
        .unwrap();

    let mut scene = scene::Scene {
        root: root_another_time,
        view_tree: taffy,
        views: {
            let mut views: HashMap<_, Box<dyn View>> = HashMap::new();

            views.insert(root_another_time, Box::new(DummyView::new("root_node")));
            views.insert(node_a, Box::new(DummyView::new("node_a")));
            views.insert(node_b, Box::new(DummyView::new("node_b")));

            views
        },
    };

    scene.set_size(Size {
        width: points(800.0),
        height: points(600.0),
    });

    for (size, location, key) in scene.get_layout() {
        println!(
            "{:?} {:?} {:?}",
            scene.views.get(&key).unwrap(),
            size,
            location
        );
    }

    // println!("{:#?}", taffy.layout(root_another_time).unwrap());
    // println!("{:#?}", taffy.layout(root_again).unwrap());
    // println!("{:#?}", taffy.layout(root_node).unwrap());
    // println!("{:#?}", taffy.layout(node_a).unwrap());
    // println!("{:#?}", taffy.layout(node_b).unwrap());

    // assert_eq!(taffy.layout(root_node).unwrap().size.width, 800.0);
    // assert_eq!(taffy.layout(root_node).unwrap().size.height, 600.0);
    // assert_eq!(taffy.layout(header_node).unwrap().size.width, 800.0);
    // assert_eq!(taffy.layout(header_node).unwrap().size.height, 100.0);
    // assert_eq!(taffy.layout(body_node).unwrap().size.width, 800.0);
    // assert_eq!(taffy.layout(body_node).unwrap().size.height, 500.0);

    // assert_eq!(taffy.layout(root_node).unwrap().location.x, 0.0);
}
