use wgpu::{CommandEncoder, TextureView};
use winit::dpi::PhysicalSize;

use super::view::{View, ViewBuilder};

#[derive(Debug, Clone)]
pub struct SolidColorView {
    color: wgpu::Color,
}

impl ViewBuilder for SolidColorView {
    fn build(
        self: Box<Self>,
        _shared_wgpu_state: std::sync::Arc<super::shared_wgpu_state::SharedWgpuState>,
        _size: PhysicalSize<u32>,
    ) -> Box<dyn View> {
        self
    }
}

impl SolidColorView {
    pub fn new(color: wgpu::Color) -> Self {
        Self { color }
    }

    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self {
            color: wgpu::Color {
                r: rng.gen(),
                g: rng.gen(),
                b: rng.gen(),
                a: 1.0,
            },
        }
    }
}

impl View for SolidColorView {
    fn set_size(&mut self, _: PhysicalSize<u32>) {}

    fn render<'pass>(
        &'pass mut self,
        command_encoder: &'pass mut CommandEncoder,
        output_view: &TextureView,
    ) {
        {
            command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
    }
}
