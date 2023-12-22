pub use core::fmt::Debug;
use std::sync::Arc;

use wgpu::{CommandEncoder, TextureView};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::WindowBuilder,
};

use super::{
    root_surface::RootSurface,
    shared_wgpu_state::{self, SharedWgpuState},
};

pub trait View: Debug {
    fn set_size(&mut self, size: PhysicalSize<u32>);
    fn render<'a>(&'a mut self, command_encoder: &'a mut CommandEncoder, output_view: &TextureView);
}

pub trait ViewBuilder {
    fn build(
        self: Box<Self>,
        shared_wgpu_state: Arc<SharedWgpuState>,
        size: PhysicalSize<u32>,
    ) -> Box<dyn View>;
}

pub async fn run(view_builder: Box<dyn ViewBuilder>) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let shared_wgpu_state = shared_wgpu_state::SharedWgpuState::new(window).await;
    let view = view_builder.build(
        shared_wgpu_state.clone(),
        shared_wgpu_state.window.inner_size(),
    );

    let mut view_surface = RootSurface::new(view, shared_wgpu_state.clone());

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
