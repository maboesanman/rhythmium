use core::fmt::Debug;
use std::collections::{HashMap, VecDeque};

use slotmap::DefaultKey;
use taffy::{geometry::Point, prelude::*};

use self::view::{SolidColorView, View};

pub mod image_view;
// pub mod root_renderer;
pub mod shared_wgpu_state;
pub mod view;
// pub mod scene_renderer;
pub mod view_surface;
pub mod scene_view;

pub struct Scene {
    pub view_tree: Taffy,
    pub root: DefaultKey,
}

impl Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene").finish()
    }
}

impl Scene {
    pub fn new(
        view_tree: Taffy,
        root: DefaultKey,
    ) -> Self {
        Self {
            view_tree,
            root,
        }
    }

    pub fn resize(&mut self, size: Size<f32>) {
        self.view_tree.compute_layout(self.root, Size {
            width: AvailableSpace::Definite(size.width),
            height: AvailableSpace::Definite(size.height),
        }).unwrap();
    }

    pub fn get_layout(&self) -> impl IntoIterator<Item = (Size<f32>, Point<f32>, DefaultKey)> {
        let root = self.root;
        // (node, global_location of parent)
        let mut queue = VecDeque::new();
        queue.push_back((
            root,
            Point {
                x: 0.0f32,
                y: 0.0f32,
            },
        ));

        let mut out: Vec<(Size<f32>, Point<f32>, DefaultKey)> = vec![];

        while let Some((key, location)) = queue.pop_front() {
            let layout = self.view_tree.layout(key).unwrap();
            let key_loc = layout.location;
            let location = Point {
                x: location.x + key_loc.x,
                y: location.y + key_loc.y,
            };

            out.push((layout.size, location, key));

            let new_entries = self
                .view_tree
                .children(key)
                .unwrap()
                .into_iter()
                .map(|child| (self.view_tree.layout(child).unwrap().order, child));

            for (_, child) in new_entries {
                queue.push_front((child, location));
            }
        }

        out
    }
}
