pub use core::fmt::Debug;
use std::{
    sync::Arc, task::Poll, time::{Duration, Instant}
};

use futures::{FutureExt, future::BoxFuture};
use rust_cef::functions::message_loop::do_message_loop_work;
use wgpu::{CommandEncoder, TextureView};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy},
    window::WindowAttributes,
};

use super::{
    root_surface::RootSurface,
    shared_wgpu_state::{self, SharedWgpuState},
};

pub struct RefreshToken(pub usize);

pub trait View: Debug {
    /// set the size of the view.
    fn set_size(&mut self, size: PhysicalSize<u32>);

    /// request a refresh of content.
    /// 
    /// if there is still an active refresh, Err(frame_number) is returned, otherwise a refresh is triggered
    /// and the returned future will be woken when the next call to render would use the new frame
    fn request_refresh(&mut self) -> Result<BoxFuture<'static, RefreshToken>, usize>;

    /// complete a refresh that was triggered by a call to request_refresh
    /// 
    /// Ok indicates you've successfully incremented the frame
    /// Err indicates you've used an invalid token
    fn complete_refresh<'pass>(&'pass mut self, command_encoder: &'pass mut CommandEncoder, refresh_token: RefreshToken) -> anyhow::Result<()>;

    /// determine the current frame counter
    fn get_current_frame(&mut self) -> usize;

    /// returns the frame number that was rendered.
    fn render<'pass>(&'pass mut self, command_encoder: &'pass mut CommandEncoder, output_view: &TextureView) -> usize;
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
    ReadyToInitialize {
        builder: Box<dyn ViewBuilder>,
        event_loop_proxy: EventLoopProxy<RhythmiumEvent>,
    },
    Uninitialized {
        builder: Box<dyn ViewBuilder>,
        event_loop_proxy: EventLoopProxy<RhythmiumEvent>,
    },
    #[default]
    Initializing,
}

pub struct ActiveViewInit {
    shared_wgpu_state: Arc<SharedWgpuState>,
    surface: RootSurface,
    next_cef_work: Option<Instant>,
    render_future: Option<BoxFuture<'static, RefreshToken>>,
    event_loop_proxy: EventLoopProxy<RhythmiumEvent>,
}

pub struct ActiveView {
    inner: ActiveViewInner,
}

#[derive(Debug, Clone)]
pub enum RhythmiumEvent {
    DoCefWorkNow,
    DoCefWorkLater(u64),
    PollRenderFuture,
}

impl ActiveView {
    pub fn new(view_builder: impl ViewBuilder + 'static, event_loop_proxy: EventLoopProxy<RhythmiumEvent>) -> Self {
        let builder = Box::new(view_builder);
        Self {
            inner: ActiveViewInner::Uninitialized {
                builder,
                event_loop_proxy,
            },
        }
    }

    pub fn ready_init(&mut self) {
        let (builder, event_loop_proxy) = match core::mem::take(&mut self.inner) {
            ActiveViewInner::Uninitialized{
                builder,
                event_loop_proxy, } => (builder, event_loop_proxy),
            _ => panic!("attempted to initialize an already initialized view"),
        };

        self.inner = ActiveViewInner::ReadyToInitialize {
            builder,
            event_loop_proxy,
        };
    }

    pub fn try_init(&mut self, event_loop: &ActiveEventLoop) {
        let (builder, event_loop_proxy) = match core::mem::take(&mut self.inner) {
            ActiveViewInner::ReadyToInitialize {
                builder,
                event_loop_proxy, } => (builder, event_loop_proxy),
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
            render_future: None,
            event_loop_proxy,
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

        if let Some(instant) = *next_cef_work
            && Instant::now() >= instant
        {
            do_message_loop_work();
            *next_cef_work = None;
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
                println!("redraw");
                let active_view_init = self.assume_init_mut();
                if active_view_init.render_future.is_some() {
                    return;
                }
                let Ok(fut) = active_view_init.surface.request_refresh() else {
                    return;
                };

                active_view_init.render_future = Some(fut);

                active_view_init.event_loop_proxy.send_event(RhythmiumEvent::PollRenderFuture).unwrap();
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

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
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
            RhythmiumEvent::PollRenderFuture => {
                println!("POLLRENDERFUTURE 1");
                let active_view_init = self.assume_init_mut();
                if let Some(fut) = &mut active_view_init.render_future {
                    println!("POLLRENDERFUTURE 2");
                    let proxy_waker = Arc::new(ProxyWaker {
                        proxy: active_view_init.event_loop_proxy.clone(),
                    });
                    let waker = futures::task::waker_ref(&proxy_waker);
                    let mut cx = std::task::Context::from_waker(&*waker);
                    if let Poll::Ready(token) = fut.poll_unpin(&mut cx) {
                        println!("POLLRENDERFUTURE 3");
                        active_view_init.surface.complete_refresh(token);
                        active_view_init.surface.render().unwrap();
                        active_view_init.shared_wgpu_state.window.request_redraw();
                    }
                }
            },
        }
    }
}

#[derive(Clone)]
struct ProxyWaker {
    proxy: winit::event_loop::EventLoopProxy<RhythmiumEvent>,
}

impl futures::task::ArcWake for ProxyWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Try to send an event that tells winit to poll again.
        // Ignore errors (they just mean the event loop is closed).
        let _ = arc_self.proxy.send_event(RhythmiumEvent::PollRenderFuture);
    }
}
