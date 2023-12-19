pub use core::fmt::Debug;
use std::{borrow::Cow, sync::Arc};

use serde::Deserialize;
use taffy::prelude::*;
use wgpu::{CommandBuffer, CommandEncoder, RenderPass, TextureView};
use winit::{event_loop::EventLoop, window::WindowBuilder, event::{Event, WindowEvent, KeyEvent, ElementState}, keyboard::NamedKey, dpi::PhysicalSize};

use super::shared_wgpu_state::{self, SharedWgpuState};

pub trait View: Debug {
    fn set_size(&mut self, size: PhysicalSize<u32>);
    fn render<'a, 'out>(
        &'a mut self,
        command_encoder: &'a mut CommandEncoder,
        output_view: &'out TextureView,
    );
}

pub struct ViewSurface {
    view: Box<dyn View>,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    shared_wgpu_state: Arc<SharedWgpuState>,
}

impl ViewSurface {
    pub fn new_root(
        view: Box<dyn View>,
        shared_wgpu_state: Arc<SharedWgpuState>,
    ) -> Self {
        Self::new(view, shared_wgpu_state.window.inner_size(), wgpu::TextureUsages::RENDER_ATTACHMENT, shared_wgpu_state)
    }
    pub fn new(
        view: Box<dyn View>,
        size: PhysicalSize<u32>,
        usage: wgpu::TextureUsages,
        shared_wgpu_state: Arc<SharedWgpuState>,
    ) -> Self {
        let instance = &shared_wgpu_state.instance;
        let window = &shared_wgpu_state.window;
        let adapter = &shared_wgpu_state.adapter;

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        Self {
            surface,
            config,
            view,
            shared_wgpu_state,
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        
        // set the surface size
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface
            .configure(&self.shared_wgpu_state.device, &self.config);

        // set the view size
        self.view.set_size(size);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.shared_wgpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        self.view.render(&mut encoder, &output_view);

        let command_buffer = encoder.finish();
        
        self.shared_wgpu_state.queue.submit([command_buffer]);
        output.present();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DummyView {
    pub name: String,
    pub size: Size<f32>,
    color: wgpu::Color,
}

impl DummyView {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            size: Size {
                width: 0.0f32,
                height: 0.0f32,
            },
            color: wgpu::Color {
                r: rand::random(),
                g: rand::random(),
                b: rand::random(),
                a: 1.0,
            },
        }
    }
}

impl View for DummyView {
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

pub async fn run(view: Box<dyn View>) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let shared_wgpu_state = shared_wgpu_state::SharedWgpuState::new(window).await;

    let mut view_surface = ViewSurface::new_root(
        view,
        shared_wgpu_state.clone(),
    );

    event_loop
        .run(move |event, window_target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == shared_wgpu_state.window.id() => match event {
                WindowEvent::RedrawRequested => {
                    view_surface.render().unwrap();
                },
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
                WindowEvent::ScaleFactorChanged { .. } => {

                }
                _ => {}
            },
            _ => {}
        })
        .unwrap();
}