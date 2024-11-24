pub use core::fmt::Debug;
use std::{sync::Arc, time::{Duration, Instant}};

use rust_cef::functions::message_loop::do_message_loop_work;
use wgpu::{CommandEncoder, TextureView};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
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
    next_cef_work: Option<Instant>,
}

pub struct ActiveView {
    inner: ActiveViewInner,
}

impl ActiveView {
    pub fn new(view_builder: impl ViewBuilder + 'static) -> Self {
        let view_builder = Box::new(view_builder);
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
            next_cef_work: None,
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
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.try_init(event_loop);

        let next_cef_work = &mut self.assume_init_mut().next_cef_work;

        if let Some(instant) = *next_cef_work {
            if Instant::now() >= instant {
                do_message_loop_work();
                *next_cef_work = None;
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(instant) = self.assume_init().next_cef_work {
            event_loop.set_control_flow(ControlFlow::WaitUntil(instant));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let active_view_init = self.assume_init_mut();
                active_view_init.surface.render().unwrap();

                // request the next frame right away.
                active_view_init.shared_wgpu_state.window.request_redraw();
            }
            WindowEvent::Resized(size) => {
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
        _event_loop: &ActiveEventLoop,
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
            RhythmiumEvent::DoCefWorkLater(milliseconds) => {
                let next_cef_work = &mut self.assume_init_mut().next_cef_work;
                if milliseconds < 1 {
                    *next_cef_work = None;
                    do_message_loop_work();
                } else {
                    *next_cef_work = Some(Instant::now() + Duration::from_millis(milliseconds));
                }
            }
            RhythmiumEvent::RenderFrame => {},
        }
    }
}
