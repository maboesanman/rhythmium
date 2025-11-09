use futures::{FutureExt as _, future::BoxFuture};
use wgpu::{CommandEncoder, TextureView};
use winit::dpi::PhysicalSize;

use crate::scene::view::RefreshToken;

use super::view::{View, ViewBuilder};

#[derive(Debug, Clone)]
pub struct SolidColorView {
    color: wgpu::Color,
    frame_count: usize,
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
        Self { color, frame_count: 0 }
    }

    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self {
            color: wgpu::Color {
                r: rng.r#gen(),
                g: rng.r#gen(),
                b: rng.r#gen(),
                a: 1.0,
            },
            frame_count: 0
        }
    }
}

impl View for SolidColorView {
    fn set_size(&mut self, _: PhysicalSize<u32>) {}

    fn render<'pass>(
        &'pass mut self,
        command_encoder: &'pass mut CommandEncoder,
        output_view: &TextureView,
    ) -> usize {
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

        self.frame_count
    }
    
    fn request_refresh(&mut self) -> Result<BoxFuture<'static, super::view::RefreshToken>, usize> {
        let fut = std::future::ready(RefreshToken(self.frame_count + 1));
        Ok(fut.boxed())
    }
    
    fn complete_refresh<'pass>(
        &'pass mut self,
        _command_encoder: &'pass mut CommandEncoder,refresh_token: super::view::RefreshToken) -> anyhow::Result<()> {
        if refresh_token.0 != self.frame_count + 1 {
            return Err(anyhow::anyhow!("invalid refresh token"));
        }

        self.frame_count += 1;
        Ok(())
    }
    
    fn get_current_frame(&mut self) -> usize {
        self.frame_count
    }
}
