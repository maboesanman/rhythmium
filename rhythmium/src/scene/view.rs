pub use core::fmt::Debug;
use std::{sync::Arc, time::Duration};

use rust_cef::functions::message_loop::do_message_loop_work;
use wgpu::{CommandEncoder, TextureView};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::WindowAttributes,
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

#[derive(Default)]
enum ActiveViewInner {
    Initialized(ActiveViewInit),
    ReadyToInitialize(Box<dyn ViewBuilder>),
    Uninitialized(Box<dyn ViewBuilder>),
    #[default]
    Initializing,
}

pub struct ActiveViewInit {
    shared_wgpu_state: Arc<SharedWgpuState>,
    surface: RootSurface,
}

pub struct ActiveView {
    inner: ActiveViewInner,
}

impl ActiveView {
    pub fn new(view_builder: Box<dyn ViewBuilder>) -> Self {
        Self {
            inner: ActiveViewInner::Uninitialized(view_builder),
        }
    }

    pub fn ready_init(&mut self) {
        let builder = match core::mem::take(&mut self.inner) {
            ActiveViewInner::Uninitialized(builder) => builder,
            _ => panic!("attempted to initialize an already initialized view"),
        };

        self.inner = ActiveViewInner::ReadyToInitialize(builder);
    }

    pub fn try_init(&mut self, event_loop: &ActiveEventLoop) {
        let builder = match core::mem::take(&mut self.inner) {
            ActiveViewInner::ReadyToInitialize(builder) => builder,
            _ => return,
        };

        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("Rhythmium")
                    .with_inner_size(LogicalSize::new(800, 600)),
            )
            .unwrap();

        let size = window.inner_size();

        let shared_wgpu_state =
            futures::executor::block_on(shared_wgpu_state::SharedWgpuState::new(window));

        let view = builder.build(shared_wgpu_state.clone(), size);
        let surface = RootSurface::new(view, shared_wgpu_state.clone());

        self.inner = ActiveViewInner::Initialized(ActiveViewInit {
            shared_wgpu_state,
            surface,
        });
    }

    pub fn assume_init(&self) -> &ActiveViewInit {
        match &self.inner {
            ActiveViewInner::Initialized(init) => init,
            _ => panic!("attempted to access uninitialized view"),
        }
    }

    pub fn assume_init_mut(&mut self) -> &mut ActiveViewInit {
        match &mut self.inner {
            ActiveViewInner::Initialized(init) => init,
            _ => panic!("attempted to access uninitialized view"),
        }
    }
}

impl ApplicationHandler<RhythmiumEvent> for ActiveView {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.try_init(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                println!("RedrawRequested");
                self.assume_init_mut().surface.render().unwrap();
            }
            WindowEvent::Resized(size) => {
                println!("Resized");
                let active_view_init = self.assume_init_mut();
                active_view_init.surface.resize(size);
                active_view_init.shared_wgpu_state.window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if cause == winit::event::StartCause::Init {
            self.ready_init();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RhythmiumEvent) {
        match event {
            RhythmiumEvent::DoCefWorkNow => {
                do_message_loop_work();
            }
            RhythmiumEvent::DoCefWorkLater(_) => {
                panic!()
            }
            RhythmiumEvent::CatchUpOnCefWork => loop {
                let start = std::time::Instant::now();
                do_message_loop_work();
                if start.elapsed() < Duration::from_micros(500) {
                    break;
                }
            },
            _ => {}
        }
    }
}
