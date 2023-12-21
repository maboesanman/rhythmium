pub use core::fmt::Debug;
use std::{borrow::Cow, sync::Arc};

use serde::Deserialize;
use taffy::prelude::*;
use wgpu::{CommandBuffer, CommandEncoder, RenderPass, TextureView};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::WindowBuilder,
};

use super::{
    shared_wgpu_state::{self, SharedWgpuState},
    view_surface::ViewSurface,
};

pub trait View: Debug {
    fn set_size(&mut self, size: PhysicalSize<u32>);
    fn render<'a, 'out>(
        &'a mut self,
        command_encoder: &'a mut CommandEncoder,
        output_view: &'out TextureView,
    );
}

#[derive(Debug, Clone)]
pub struct SolidColorView {
    color: wgpu::Color,
}

impl SolidColorView {
    pub fn new() -> Self {
        Self {
            color: wgpu::Color {
                r: rand::random(),
                g: rand::random(),
                b: rand::random(),
                a: 1.0,
            },
        }
    }
}

impl View for SolidColorView {
    fn set_size(&mut self, _: PhysicalSize<u32>) {}

    fn render<'pass, 'out>(
        &'pass mut self,
        command_encoder: &'pass mut CommandEncoder,
        output_view: &'out TextureView,
    ) {
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

pub async fn run<F>(view_callback: F)
where
    F: FnOnce(Arc<SharedWgpuState>) -> Box<dyn View>,
{
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let shared_wgpu_state = shared_wgpu_state::SharedWgpuState::new(window).await;
    let view = view_callback(shared_wgpu_state.clone());

    let mut view_surface = ViewSurface::new_root(view, shared_wgpu_state.clone());

    event_loop
        .run(move |event, window_target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == shared_wgpu_state.window.id() => match event {
                WindowEvent::RedrawRequested => {
                    view_surface.render().unwrap();
                }
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key: winit::keyboard::Key::Named(NamedKey::Escape),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    window_target.exit();
                }
                WindowEvent::Resized(..) => {
                    let size = shared_wgpu_state.window.inner_size();
                    view_surface.resize(size);
                    shared_wgpu_state.window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {}
                _ => {}
            },
            _ => {}
        })
        .unwrap();
}
