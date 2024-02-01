pub use core::fmt::Debug;
use std::sync::Arc;

use cef_wrapper::do_cef_message_loop_work;
use wgpu::{CommandEncoder, TextureView};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, StartCause, WindowEvent},
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::WindowBuilder,
};

use crate::RhythmiumEvent;

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

pub fn run(event_loop: EventLoop<RhythmiumEvent>, view_builder: Box<dyn ViewBuilder>) {
    let window = WindowBuilder::new()
        .with_title("Rhythmium")
        .build(&event_loop)
        .unwrap();

    let shared_wgpu_state = futures::executor::block_on(shared_wgpu_state::SharedWgpuState::new(window));
    let view = view_builder.build(
        shared_wgpu_state.clone(),
        shared_wgpu_state.window.inner_size(),
    );

    let size = shared_wgpu_state.window.inner_size();
    let mut view_surface = RootSurface::new(view, shared_wgpu_state.clone());
    view_surface.resize(size);

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop
        .run(move |event, window_target| {
            match event {
                Event::AboutToWait => {
                    view_surface.render().unwrap();
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == shared_wgpu_state.window.id() => match event {
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
                    }
                    _ => {}
                },
                Event::UserEvent(RhythmiumEvent::DoCefWorkNow) => {
                    do_cef_message_loop_work();
                },
                Event::UserEvent(RhythmiumEvent::DoCefWorkLater(t)) => {
                    panic!()
                }
                _ => {}
            };
        })
        .unwrap();
}
