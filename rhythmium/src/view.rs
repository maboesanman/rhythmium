pub use core::fmt::Debug;

use serde::Deserialize;
use taffy::prelude::*;

pub trait View: Debug {
    fn set_size(&mut self, size: Size<f32>);
    fn get_size(&self) -> Size<f32>;
}

#[derive(Debug, Clone)]
pub struct DummyView {
    pub name: String,
    pub size: Size<f32>,
}

impl DummyView {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            size: Size {
                width: 0.0f32,
                height: 0.0f32,
            },
        }
    }
}

impl View for DummyView {
    fn set_size(&mut self, size: Size<f32>) {
        self.size = size;
    }

    fn get_size(&self) -> Size<f32> {
        self.size
    }
}

#[derive(Deserialize, Debug)]
pub struct ViewBuilder {
    pub name: String,
    pub description: String,
    pub id: String,
}

impl ViewBuilder {
    pub fn build(self) -> Box<dyn View> {
        Box::new(DummyView::new(&self.name))
    }
}
