use std::sync::Arc;

use winit::dpi::PhysicalSize;

use super::{shared_wgpu_state::SharedWgpuState, view::View};

pub struct RootSurface {
    view: Box<dyn View>,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    shared_wgpu_state: Arc<SharedWgpuState>,
}

impl RootSurface {
    pub fn new(view: Box<dyn View>, shared_wgpu_state: Arc<SharedWgpuState>) -> Self {
        let size = shared_wgpu_state.window.inner_size();
        let instance = &shared_wgpu_state.instance;
        let window = &shared_wgpu_state.window;
        let adapter = &shared_wgpu_state.adapter;

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let surface_capabilities = surface.get_capabilities(adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
