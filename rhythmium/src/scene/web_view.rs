use std::sync::{Arc, RwLock};

use cef_wrapper::{browser_host::BrowserHost, CefApp};
use wgpu::{util::DeviceExt, CommandEncoder, TextureView};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};

use super::{
    shared_wgpu_state::SharedWgpuState,
    view::{View, ViewBuilder},
};

pub struct WebViewBuilder {
    cef_app: Arc<CefApp>,
}

impl WebViewBuilder {
    pub fn new(cef_app: Arc<CefApp>) -> Self {
        Self { cef_app }
    }
}

impl ViewBuilder for WebViewBuilder {
    fn build(
        self: Box<Self>,
        shared_wgpu_state: Arc<SharedWgpuState>,
        size: PhysicalSize<u32>,
    ) -> Box<dyn View> {
        let fut = WebView::new(shared_wgpu_state, size, &self.cef_app);
        let view = futures::executor::block_on(fut);
        Box::new(view)
    }
}

#[derive(Debug)]
pub struct WebView {
    shared_wgpu_state: Arc<SharedWgpuState>,
    texture_bind_group: Arc<RwLock<wgpu::BindGroup>>,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    size: Arc<RwLock<PhysicalSize<u32>>>,

    browser_host: BrowserHost,

    current_scale_factor: f64,
}

impl WebView {
    pub async fn new(
        shared_wgpu_state: Arc<SharedWgpuState>,
        size: PhysicalSize<u32>,
        cef_app: &CefApp,
    ) -> Self {
        let device = &shared_wgpu_state.device;
        let queue = &shared_wgpu_state.queue;

        let texture_size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        let mut texture = device.create_texture(&wgpu::TextureDescriptor {
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
                texture: &texture,
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

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
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
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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

        let texture_bind_group = Arc::new(RwLock::new(texture_bind_group));
        let texture_bind_group_clone = texture_bind_group.clone();

        let size = Arc::new(RwLock::new(size));
        let size_clone = size.clone();

        let shared_wgpu_state_clone = shared_wgpu_state.clone();
        let shared_wgpu_state_clone_2 = shared_wgpu_state.clone();
        let shared_wgpu_state_clone_3 = shared_wgpu_state.clone();

        let browser = cef_app
            .create_browser(
                move |w, h| {
                    let w = unsafe { w.as_mut().unwrap() };
                    let h = unsafe { h.as_mut().unwrap() };

                    let physical_size = *size_clone.read().unwrap();
                    let logical_size: LogicalSize<i32> =
                        physical_size.to_logical(shared_wgpu_state_clone.window.scale_factor());

                    *w = logical_size.width;
                    *h = logical_size.height;
                },
                move |dirty_count, dirty_start, buf, w, h| {
                    let dirty_rects = unsafe {
                        let dirty_start = dirty_start.cast::<cef_wrapper::CefRect>();
                        std::slice::from_raw_parts(dirty_start, dirty_count as usize)
                    };

                    let buf = unsafe {
                        std::slice::from_raw_parts(buf.cast::<u8>(), (w * h * 4) as usize)
                    };
                    let current_size = texture.size();
                    if current_size.width != w as u32 || current_size.height != h as u32 {
                        let texture_size = wgpu::Extent3d {
                            width: w as u32,
                            height: h as u32,
                            depth_or_array_layers: 1,
                        };

                        let new_texture = device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("WebView Texture"),
                            size: texture_size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });

                        queue.write_texture(
                            wgpu::ImageCopyTexture {
                                aspect: wgpu::TextureAspect::All,
                                texture: &new_texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                            },
                            buf,
                            wgpu::ImageDataLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * texture_size.width),
                                rows_per_image: Some(texture_size.height),
                            },
                            texture_size,
                        );

                        let texture_view =
                            new_texture.create_view(&wgpu::TextureViewDescriptor::default());
                        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                            address_mode_u: wgpu::AddressMode::ClampToEdge,
                            address_mode_v: wgpu::AddressMode::ClampToEdge,
                            address_mode_w: wgpu::AddressMode::ClampToEdge,
                            mag_filter: wgpu::FilterMode::Nearest,
                            min_filter: wgpu::FilterMode::Nearest,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            ..Default::default()
                        });

                        *texture_bind_group_clone.write().unwrap() =
                            device.create_bind_group(&wgpu::BindGroupDescriptor {
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

                        texture = new_texture;
                        return;
                    }

                    for rect in dirty_rects {
                        let texture_copy_view = wgpu::ImageCopyTexture {
                            texture: &texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: rect.x as u32,
                                y: rect.y as u32,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        };

                        let texture_data_layout = wgpu::ImageDataLayout {
                            offset: (rect.x * 4 + rect.y * w * 4) as u64,
                            bytes_per_row: Some(4 * w as u32),
                            rows_per_image: None,
                        };

                        let texture_extent = wgpu::Extent3d {
                            width: rect.width as u32,
                            height: rect.height as u32,
                            depth_or_array_layers: 1,
                        };

                        queue.write_texture(
                            texture_copy_view,
                            buf,
                            texture_data_layout,
                            texture_extent,
                        );
                    }
                },
                move |scale_factor| {
                    let scale_factor = unsafe { scale_factor.as_mut().unwrap() };
                    *scale_factor = shared_wgpu_state_clone_2.window.scale_factor() as f32;
                },
                move |view_x, view_y, screen_x, screen_y| {
                    let scale_factor = shared_wgpu_state_clone_3.window.scale_factor();

                    let logical_position = LogicalPosition::new(view_x, view_y);
                    let physical_position = logical_position.to_physical(scale_factor);

                    unsafe {
                        *screen_x = physical_position.x;
                        *screen_y = physical_position.y;
                    }
                },
            )
            .await;

        let current_scale_factor = shared_wgpu_state.window.scale_factor();

        Self {
            shared_wgpu_state,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            texture_bind_group,
            size,
            browser_host: browser.get_host(),
            current_scale_factor,
        }
    }
}

impl View for WebView {
    fn set_size(&mut self, size: PhysicalSize<u32>) {
        *self.size.write().unwrap() = size;
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
    ) {
        let texture_bind_group = self.texture_bind_group.read().unwrap();
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
            render_pass.set_bind_group(0, &texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
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
