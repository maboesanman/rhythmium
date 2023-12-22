use core::fmt::Debug;
use std::collections::VecDeque;

use slotmap::DefaultKey;
use taffy::{geometry::Point, prelude::*};

pub mod image_view;
pub mod root_surface;
pub mod scene_view;
pub mod shared_wgpu_state;
pub mod solid_color_view;
pub mod view;

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
    #[must_use]
    pub fn new(view_tree: Taffy, root: DefaultKey) -> Self {
        Self { view_tree, root }
    }

    pub fn resize(&mut self, size: Size<f32>) {
        self.view_tree
            .compute_layout(
                self.root,
                Size {
                    width: AvailableSpace::Definite(size.width),
                    height: AvailableSpace::Definite(size.height),
                },
            )
            .unwrap();
    }

    #[must_use]
    pub fn get_layout(&self) -> impl '_ + IntoIterator<Item = (Size<f32>, Point<f32>, DefaultKey)> {
        LayoutIter {
            scene: self,
            queue: VecDeque::from(vec![(self.root, Point { x: 0.0, y: 0.0 })]),
        }
    }
}

struct LayoutIter<'a> {
    scene: &'a Scene,
    queue: VecDeque<(DefaultKey, Point<f32>)>,
}

impl Iterator for LayoutIter<'_> {
    type Item = (Size<f32>, Point<f32>, DefaultKey);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, location) = self.queue.pop_front()?;
        let layout = self.scene.view_tree.layout(key).unwrap();
        let key_loc = layout.location;
        let location = Point {
            x: location.x + key_loc.x,
            y: location.y + key_loc.y,
        };

        let new_entries = self
            .scene
            .view_tree
            .children(key)
            .unwrap()
            .into_iter()
            .map(|child| (self.scene.view_tree.layout(child).unwrap().order, child));

        for (_, child) in new_entries {
            self.queue.push_front((child, location));
        }

        Some((layout.size, location, key))
    }
}
