use std::{borrow::Cow, sync::Arc};

use image::GenericImageView;
use wgpu::{util::DeviceExt, CommandEncoder, PipelineCompilationOptions, TextureView};
use winit::dpi::PhysicalSize;

use super::{
    shared_wgpu_state::SharedWgpuState,
    view::{View, ViewBuilder},
};
use anyhow::*;

pub struct ImageViewBuilder<'a> {
    image_bytes: &'a [u8],
    fit: ImageFit,
}

impl<'a> ImageViewBuilder<'a> {
    pub fn new(image_bytes: &'a [u8], fit: ImageFit) -> Self {
        Self { image_bytes, fit }
    }
}

impl<'a> ViewBuilder for ImageViewBuilder<'a> {
    fn build(
        self: Box<Self>,
        shared_wgpu_state: Arc<SharedWgpuState>,
        size: PhysicalSize<u32>,
    ) -> Box<dyn View> {
        Box::new(ImageView::new(
            shared_wgpu_state,
            self.image_bytes,
            size,
            self.fit,
        ))
    }
}

#[derive(Debug)]
pub struct ImageView {
    shared_wgpu_state: Arc<SharedWgpuState>,
    texture: Arc<Texture>,
    size: PhysicalSize<u32>,
    fit: ImageFit,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    diffuse_bind_group: wgpu::BindGroup,
}

impl View for ImageView {
    fn set_size(&mut self, size: PhysicalSize<u32>) {
        self.size = size;

        self.vertex_buffer = get_vertex_buffer(
            &self.shared_wgpu_state.device,
            self.fit,
            size.height as f32 / size.width as f32,
            self.texture.texture.size().height as f32 / self.texture.texture.size().width as f32,
        );
    }

    fn render<'pass>(
        &'pass mut self,
        command_encoder: &'pass mut CommandEncoder,
        output_view: &TextureView,
    ) {
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Image View Render Pass"),
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
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
    }
}

impl ImageView {
    fn new(
        shared_wgpu_state: Arc<SharedWgpuState>,
        image_bytes: &[u8],
        size: PhysicalSize<u32>,
        fit: ImageFit,
    ) -> Self {
        let texture = Texture::from_bytes(&shared_wgpu_state, image_bytes, None).unwrap();

        let device = &shared_wgpu_state.device;

        let shader = device.create_shader_module(wgpu::include_wgsl!("image_view.wgsl"));

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
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

        let vertex_buffer = get_vertex_buffer(
            &shared_wgpu_state.device,
            fit,
            1.0,
            texture.texture.size().height as f32 / texture.texture.size().width as f32,
        );

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image View Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Diffuse Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        Self {
            shared_wgpu_state,
            texture: Arc::new(texture),
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            diffuse_bind_group,
            fit,
        }
    }
}
// impl View for ImageView {

// }

// let Vertex

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(
        shared_wgpu_state: &SharedWgpuState,
        bytes: &[u8],
        label: Option<&str>,
    ) -> Result<Self> {
        let image = image::load_from_memory(bytes)?;
        Self::from_image(shared_wgpu_state, &image, label)
    }

    pub fn from_image(
        shared_wgpu_state: &SharedWgpuState,
        image: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let dimensions = image.dimensions();

        let device = &shared_wgpu_state.device;
        let queue = &shared_wgpu_state.queue;

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            &image.to_rgba8(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

const VERTICES_FULL: &[Vertex] = &[
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

pub fn get_vertex_buffer(
    device: &wgpu::Device,
    fit: ImageFit,
    container_ratio: f32, // height / width
    content_ratio: f32,   // height / width
) -> wgpu::Buffer {
    let fit = match fit {
        ImageFit::Contain => {
            if container_ratio > content_ratio {
                ImageFit::SetWidth(ImageJustification::Center)
            } else {
                ImageFit::SetHeight(ImageJustification::Center)
            }
        }
        ImageFit::Cover => {
            if container_ratio < content_ratio {
                ImageFit::SetWidth(ImageJustification::Center)
            } else {
                ImageFit::SetHeight(ImageJustification::Center)
            }
        }
        fit => fit,
    };

    let vertices = match fit {
        ImageFit::Stretch => Cow::Borrowed(VERTICES_FULL),
        ImageFit::SetWidth(just) => {
            let new_y = content_ratio / container_ratio;
            let mut vertices = VERTICES_FULL.to_owned();
            for Vertex {
                position: [_, y], ..
            } in vertices.iter_mut()
            {
                let adjust = match just {
                    ImageJustification::Start => 1.0 - new_y,
                    ImageJustification::Center => 0.0,
                    ImageJustification::End => new_y - 1.0,
                };

                *y = *y * new_y + adjust;
            }

            Cow::Owned(vertices)
        }
        ImageFit::SetHeight(just) => {
            let new_x = container_ratio / content_ratio;
            let mut vertices = VERTICES_FULL.to_owned();
            for Vertex {
                position: [x, _], ..
            } in vertices.iter_mut()
            {
                let adjust = match just {
                    ImageJustification::Start => new_x - 1.0,
                    ImageJustification::Center => 0.0,
                    ImageJustification::End => 1.0 - new_x,
                };

                *x = *x * new_x + adjust;
            }

            Cow::Owned(vertices)
        }
        _ => unreachable!(),
    };

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Image View Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFit {
    Stretch,
    Contain,
    Cover,
    SetWidth(ImageJustification),
    SetHeight(ImageJustification),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageJustification {
    Start,
    Center,
    End,
}
