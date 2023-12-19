use std::sync::Arc;

use image::GenericImageView as _;
use wgpu::util::DeviceExt;
use wgpu::Device;
use winit::window::Window;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    keyboard::NamedKey,
    window::WindowBuilder,
};

use crate::scene::{shared_wgpu_state, Scene};

use super::image_view::ImageView;
use super::view::{View, ViewSurface};

struct RootRenderer {
    view_surface: ViewSurface,
    shared_wgpu_state: Arc<super::shared_wgpu_state::SharedWgpuState>,
}

impl RootRenderer {
    async fn new(mut scene: Scene, window: Window) -> Self {

        let view_surface = ViewSurface::new(
            View::new(
                scene.root,
                scene.view_tree,
                scene.views,
                scene.view_tree.layout(scene.root).unwrap(),
            ),
            shared_wgpu_state.clone(),
        );

        Self {
            view_surface,
            shared_wgpu_state,
        }
    }

    pub fn window(&self) -> &Window {
        &self.shared_wgpu_state.window
    }

    fn resize(&mut self) {
        let new_size = self.shared_wgpu_state.window.inner_size();

        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        // set the surface size
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface
            .configure(&self.shared_wgpu_state.device, &self.surface_config);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.shared_wgpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // render_pass.set_pipeline(&self.render_pipeline);
            // render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            // render_pass.draw(0..NUM_VERTICES, 0..1);
        }

        self.shared_wgpu_state
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

