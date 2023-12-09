use std::collections::{HashMap, VecDeque};
use core::fmt::Debug;

use serde::Deserialize;
use slotmap::DefaultKey;
use taffy::{geometry::Point, prelude::*};

use crate::view::{View, ViewBuilder};

pub struct Scene {
    pub view_tree: Taffy,
    pub root: DefaultKey,
    pub views: HashMap<DefaultKey, Box<dyn View>>,
}

impl Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene").finish()
    }
}

impl View for Scene {
    fn set_size(&mut self, size: Size<f32>) {
        self.view_tree.compute_layout(self.root, Size {
            width: AvailableSpace::Definite(size.width),
            height: AvailableSpace::Definite(size.height),
        }).unwrap();

        for (key, view) in self.views.iter_mut() {
            let layout = self.view_tree.layout(*key).unwrap();
            view.set_size(layout.size);
        }
    }

    fn get_size(&self) -> Size<f32> {
        self.view_tree.layout(self.root).unwrap().size
    }
}

impl Scene {
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

        loop {
            let (key, location) = match queue.pop_front() {
                Some(x) => x,
                None => break,
            };

            let layout = self.view_tree.layout(key).unwrap();
            let key_loc = layout.location;
            let location = Point {
                x: location.x + key_loc.x,
                y: location.y + key_loc.y,
            };

            if self.views.contains_key(&key) {
                out.push((layout.size, location, key));
            }

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

#[derive(Deserialize, Debug)]
pub struct SceneBuilder {
    name: String,
    description: String,
    views: Vec<ViewBuilder>,
    view_layout: SceneLayout,
    // inputs: Vec<InputBuilder>,
    // pipeline_steps: Vec<PipelineStepBuilder>,
    // pipeline_sequence: Vec<PipelineSequenceBuilder>,
}

impl SceneBuilder {
    pub fn build(self) -> Scene {
        let views = self
            .views
            .into_iter()
            .map(|view| (view.id.clone(), Box::new(view.build())))
            .collect::<HashMap<_, _>>();

        let mut taffy = Taffy::new();



        todo!()
    }

}


#[derive(Deserialize, Debug)]
pub struct SceneLayout {
    name: String,
    view_id: String,
    style: Style,
    children: Vec<SceneLayout>,
}

impl SceneLayout {
    pub fn build_tree(self, &mut ) -> (DefaultKey,  {
        todo!()
    }
}