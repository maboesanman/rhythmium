use std::{num::NonZeroU32, sync::Arc};

use futures::{FutureExt, channel::oneshot};
use parking_lot::RwLock;
use rust_cef::{
    c_to_rust::{browser::Browser, browser_host::BrowserHost},
    rust_to_c::{
        client::{Client, ClientConfig},
        load_handler::{LoadHandler, LoadHandlerConfig},
        render_handler::{RenderHandler, RenderHandlerConfig},
    },
    structs::{geometry::Rect, screen_info::ScreenInfo},
    util::cef_arc::CefArc,
};
use wgpu::{CommandEncoder, Extent3d, PipelineCompilationOptions, TextureView, util::DeviceExt};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};

use crate::scene::view::RefreshToken;

use super::{
    shared_wgpu_state::SharedWgpuState,
    view::{View, ViewBuilder},
};

pub struct WebViewBuilder {}

impl Default for WebViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WebViewBuilder {
    pub fn new() -> Self {
        Self {}
    }
}

impl ViewBuilder for WebViewBuilder {
    fn build(
        self: Box<Self>,
        shared_wgpu_state: Arc<SharedWgpuState>,
        size: PhysicalSize<u32>,
    ) -> Box<dyn View> {
        let fut = WebView::new(shared_wgpu_state, size);
        let view = futures::executor::block_on(fut);
        Box::new(view)
    }
}

#[derive(Debug)]
pub struct WebView {
    shared_wgpu_state: Arc<SharedWgpuState>,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    browser_host: CefArc<BrowserHost>,

    current_scale_factor: f64,

    shared_writable: Arc<RwLock<SharedWritable>>,
}

impl WebView {
    pub async fn new(shared_wgpu_state: Arc<SharedWgpuState>, size: PhysicalSize<u32>) -> Self {
        let device = &shared_wgpu_state.device;
        let queue = &shared_wgpu_state.queue;

        let texture_size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        let working_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("WebView Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &working_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &vec![128u8; 4 * texture_size.width as usize * texture_size.height as usize],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_size.width),
                rows_per_image: Some(texture_size.height),
            },
            texture_size,
        );

        let visible_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("WebView Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &visible_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &vec![128u8; 4 * texture_size.width as usize * texture_size.height as usize],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_size.width),
                rows_per_image: Some(texture_size.height),
            },
            texture_size,
        );

        let texture_view = working_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("web_view.wgsl"));

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("WebView Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_bind_group_layout = Arc::new(texture_bind_group_layout);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Web View Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Web View Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Diffuse Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
            ],
        });

        let window_info = rust_cef::structs::window_info::WindowInfo {
            window_name: "".into(),
            bounds: Rect {
                x: 0,
                y: 0,
                width: size.width as i32,
                height: size.height as i32,
            },
            hidden: false,
            windowless_rendering_enabled: true,
            external_begin_frame_enabled: true,
        };

        let browser_settings = rust_cef::structs::browser_settings::BrowserSettings {
            windowless_frame_rate: NonZeroU32::new(60),
            ..Default::default()
        };

        let shared_writable = Arc::new(RwLock::new(SharedWritable {
            size,
            working_texture,
            visible_texture,
            texture_bind_group,
            frame_count: 0,
            notifier: None,
            dirty: false,
            loading: true,
        }));

        let client = WebViewClient::new(
            shared_wgpu_state.clone(),
            texture_bind_group_layout,
            shared_writable.clone(),
        );

        let browser = BrowserHost::create_browser_sync(
            &window_info,
            client,
            "http://webglsamples.org/aquarium/aquarium.html",
            // "https://www.google.com",
            // "https://www.youtube.com/embed/DkHDLYPa71o?autoplay=1",
            // "https://codepen.io/Alina_Niko/pen/eYqyYyP",
            &browser_settings,
        );
        let browser_host = browser.get_host();
        let current_scale_factor = shared_wgpu_state.window.scale_factor();
        // browser_host.

        Self {
            shared_wgpu_state,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            browser_host,
            current_scale_factor,
            shared_writable,
        }
    }
}

impl View for WebView {
    fn set_size(&mut self, size: PhysicalSize<u32>) {
        self.shared_writable.write().size = size;
        self.browser_host.was_resized();

        if self.current_scale_factor != self.shared_wgpu_state.window.scale_factor() {
            self.current_scale_factor = self.shared_wgpu_state.window.scale_factor();
            self.browser_host.notify_screen_info_changed();
        }
    }

    fn render<'pass>(
        &'pass mut self,
        command_encoder: &'pass mut CommandEncoder,
        output_view: &TextureView,
    ) -> usize {
        let shared_writable = self.shared_writable.read();
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Web View Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &shared_writable.texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        shared_writable.frame_count
    }

    fn request_refresh(
        &mut self,
    ) -> Result<futures::future::BoxFuture<'static, RefreshToken>, usize> {
        let (send, recv) = oneshot::channel::<RefreshToken>();
        let mut shared = self.shared_writable.write();
        if shared.notifier.is_some() {
            return Err(shared.frame_count);
        }
        shared.notifier = Some(send);
        shared.dirty = true;

        if !shared.loading {
            println!("send_external_begin_frame");
            // self.browser_host.invalidate();
            self.browser_host.send_external_begin_frame();
        }

        Ok(async { recv.await.unwrap() }.boxed())
    }

    fn complete_refresh<'pass>(
        &'pass mut self,
        command_encoder: &'pass mut CommandEncoder,
        refresh_token: super::view::RefreshToken,
    ) -> anyhow::Result<()> {
        let mut shared = self.shared_writable.write();

        command_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &shared.working_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &shared.visible_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: shared.size.width,
                height: shared.size.height,
                depth_or_array_layers: 1,
            },
        );
        shared.dirty = false;

        Ok(())
    }

    fn get_current_frame(&mut self) -> usize {
        self.shared_writable.read().frame_count
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 1, 3, 2];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

struct WebViewClient {
    render_handler: CefArc<RenderHandler>,
    load_handler: CefArc<LoadHandler>,
}

impl WebViewClient {
    pub fn new(
        shared_wgpu_state: Arc<SharedWgpuState>,
        texture_bind_group_layout: Arc<wgpu::BindGroupLayout>,
        shared_writable: Arc<RwLock<SharedWritable>>,
    ) -> CefArc<Client> {
        let render_handler = RenderHandler::new(WebViewRenderHandler {
            shared_wgpu_state: shared_wgpu_state.clone(),
            shared_writable: shared_writable.clone(),
            texture_bind_group_layout: texture_bind_group_layout.clone(),
        });

        let load_handler = LoadHandler::new(WebViewLoadHandler {
            shared_writable: shared_writable.clone(),});

        Client::new(Self {
            render_handler,
            load_handler,
        })
    }
}

impl ClientConfig for WebViewClient {
    fn get_render_handler(&self) -> Option<CefArc<RenderHandler>> {
        Some(self.render_handler.clone())
    }

    fn get_load_handler(&self) -> Option<CefArc<LoadHandler>> {
        Some(self.load_handler.clone())
    }
}

#[derive(Debug)]
struct SharedWritable {
    size: PhysicalSize<u32>,
    working_texture: wgpu::Texture,
    visible_texture: wgpu::Texture,
    texture_bind_group: wgpu::BindGroup,
    frame_count: usize,
    notifier: Option<oneshot::Sender<RefreshToken>>,
    dirty: bool,
    loading: bool,
}

struct WebViewRenderHandler {
    shared_wgpu_state: Arc<SharedWgpuState>,
    texture_bind_group_layout: Arc<wgpu::BindGroupLayout>,
    shared_writable: Arc<RwLock<SharedWritable>>,
}

impl RenderHandlerConfig for WebViewRenderHandler {
    fn get_view_rect(&mut self, _: CefArc<Browser>) -> Option<Rect> {
        println!("get_view_rect");
        let physical_size = self.shared_writable.read().size;
        let logical_size: LogicalSize<i32> =
            physical_size.to_logical(self.shared_wgpu_state.window.scale_factor());

        Some(Rect {
            x: 0,
            y: 0,
            width: logical_size.width,
            height: logical_size.height,
        })
    }

    fn on_paint(
        &mut self,
        _browser: CefArc<Browser>,
        _paint_element_type: rust_cef::enums::paint_element_type::PaintElementType,
        dirty_rects: &[Rect],
        buffer: &[u8],
        width: usize,
        height: usize,
    ) {
        println!("ONPAINT");
        let mut shared_writable = self.shared_writable.write();

        let current_size = shared_writable.working_texture.size();
        if current_size.width != width as u32 || current_size.height != height as u32 {
            let texture_size = wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            };

            shared_writable.working_texture =
                self.shared_wgpu_state
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("WebView Texture"),
                        size: texture_size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_DST
                            | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    });

            shared_writable.visible_texture =
                self.shared_wgpu_state
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("WebView Texture"),
                        size: texture_size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });

            self.shared_wgpu_state.queue.write_texture(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &shared_writable.working_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                buffer,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * texture_size.width),
                    rows_per_image: Some(texture_size.height),
                },
                texture_size,
            );

            self.shared_wgpu_state.queue.write_texture(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &shared_writable.visible_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                buffer,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * texture_size.width),
                    rows_per_image: Some(texture_size.height),
                },
                texture_size,
            );

            let texture_view = shared_writable
                .visible_texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let texture_sampler =
                self.shared_wgpu_state
                    .device
                    .create_sampler(&wgpu::SamplerDescriptor {
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Nearest,
                        min_filter: wgpu::FilterMode::Nearest,
                        mipmap_filter: wgpu::FilterMode::Nearest,
                        ..Default::default()
                    });
            shared_writable.texture_bind_group =
                self.shared_wgpu_state
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Diffuse Bind Group"),
                        layout: &self.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&texture_sampler),
                            },
                        ],
                    });

            return;
        }

        for rect in dirty_rects {
            let texture_copy_view = wgpu::ImageCopyTexture {
                texture: &shared_writable.working_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.x as u32,
                    y: rect.y as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            };

            let offset = rect.x as u64 * 4 + rect.y as u64 * width as u64 * 4;

            let texture_data_layout = wgpu::ImageDataLayout {
                offset,
                bytes_per_row: Some(4 * width as u32),
                rows_per_image: None,
            };

            let texture_extent = wgpu::Extent3d {
                width: rect.width as u32,
                height: rect.height as u32,
                depth_or_array_layers: 1,
            };

            self.shared_wgpu_state.queue.write_texture(
                texture_copy_view,
                buffer,
                texture_data_layout,
                texture_extent,
            );
        }

        shared_writable.frame_count += 1;
        let frame_count = shared_writable.frame_count;
        shared_writable
            .notifier
            .take()
            .map(|n| n.send(RefreshToken(frame_count)));
    }

    fn get_screen_info(&mut self, _browser: CefArc<Browser>) -> Option<ScreenInfo> {
        println!("get_screen_info");
        let device_scale_factor = self.shared_wgpu_state.window.scale_factor() as f32;
        let size = self.shared_writable.read().size;
        let dummy_rect = Rect {
            x: 0,
            y: 0,
            width: size.width as i32,
            height: size.height as i32,
        };
        Some(ScreenInfo {
            device_scale_factor,
            depth: 32,
            depth_per_component: 8,
            is_monochrome: false,
            rect: dummy_rect,
            available_rect: dummy_rect,
        })
    }

    fn get_screen_point(
        &mut self,
        _browser: CefArc<Browser>,
        view_x: i32,
        view_y: i32,
    ) -> Option<(i32, i32)> {
        let scale_factor = self.shared_wgpu_state.window.scale_factor();
        let logical_position = LogicalPosition::new(view_x, view_y);
        let physical_position = logical_position.to_physical(scale_factor);

        Some((physical_position.x, physical_position.y))
    }
}

struct WebViewLoadHandler {
    shared_writable: Arc<RwLock<SharedWritable>>,}

impl LoadHandlerConfig for WebViewLoadHandler {
    fn on_load_end(&mut self, browser: CefArc<Browser>, status_code: u32) {
        println!("LOAD_END: {status_code}");
        let mut shared = self.shared_writable.write();
        shared.loading = false;
        browser.get_host().was_resized();
        browser.get_host().invalidate();
        if shared.dirty {
            println!("send_external_begin_frame");
            browser.get_host().send_external_begin_frame();
        }
    }
}
