use std::{borrow::Cow, sync::Arc, collections::HashMap};

use image::GenericImageView;
use slotmap::DefaultKey;
use taffy::Taffy;
use wgpu::{util::DeviceExt, CommandEncoder, TextureView};
use winit::dpi::PhysicalSize;

use super::{shared_wgpu_state::SharedWgpuState, view::View, Scene};

#[derive(Debug)]
pub struct SceneView {
    size: PhysicalSize<u32>,
    
    scene: Scene,
    views: HashMap<DefaultKey, SceneSubView>,
    
    index_buffer: wgpu::Buffer,
    
    render_pipeline: wgpu::RenderPipeline,
    _shared_wgpu_state: Arc<SharedWgpuState>,
}

#[derive(Debug)]
struct SceneSubView {
    view: Box<dyn View>,
    texture_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    shared_wgpu_state: Arc<SharedWgpuState>,
}

impl SceneSubView {
    pub fn new(
        view: Box<dyn View>,
        size: PhysicalSize<u32>,
        bind_group_layout: &wgpu::BindGroupLayout,
        shared_wgpu_state: Arc<SharedWgpuState>,
    ) -> Self {
        let texture = Self::get_texture(size, &shared_wgpu_state);
        let sampler = shared_wgpu_state.device.create_sampler(&Default::default());
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Bgra8UnormSrgb),
            ..Default::default()
        });
        let bind_group = shared_wgpu_state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            view,
            texture_view,
            bind_group,
            shared_wgpu_state,
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.view.set_size(size);
    }

    fn get_texture(size: PhysicalSize<u32>, shared_wgpu_state: &SharedWgpuState) -> wgpu::Texture {
        shared_wgpu_state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Sub View Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }
}

impl View for SceneView {
    fn set_size(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        self.scene.resize(taffy::geometry::Size {
            width: size.width as f32,
            height: size.height as f32,
        });

        for (size, _, key) in self.scene.get_layout() {
            let view = match self.views.get_mut(&key) {
                Some(view) => view,
                None => continue,
            };
            view.resize(PhysicalSize {
                width: size.width as u32,
                height: size.height as u32,
            });
        }
    }

    fn render<'pass>(
        &'pass mut self,
        command_encoder: &'pass mut CommandEncoder,
        output_view: &TextureView,
    ) {
        {
            let layout: Vec<_> = self.scene.get_layout().into_iter().filter_map(|(size, position, key)| {
                let sub_view = match self.views.get_mut(&key) {
                    Some(view) => view,
                    None => return None,
                };
                
                sub_view.view.render(command_encoder, &sub_view.texture_view);
                let x = position.x;
                let y = position.y;
                let w = size.width;
                let h = size.height;

                let x = x * 2.0 / self.size.width as f32 - 1.0;
                let y = y * 2.0 / self.size.height as f32 - 1.0;
                let w = w * 2.0 / self.size.width as f32;
                let h = h * 2.0 / self.size.height as f32;
                let mut vertices = SET_TEX_COORDS.clone();
                vertices[0].position = [x, y];
                vertices[1].position = [x + w, y];
                vertices[2].position = [x, y + h];
                vertices[3].position = [x + w, y + h];

                let vertex_buffer = sub_view
                    .shared_wgpu_state
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                
                Some((key, vertex_buffer))
            }).collect();

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            
            for (key, vertex_buffer) in layout.iter() {
                let sub_view = self.views.get(&key).unwrap();
                render_pass.set_bind_group(0, &sub_view.bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw_indexed(0..6, 0, 0..1);
            }
        }
    }
}

impl SceneView {
    pub fn new(
        scene: Scene,
        views: HashMap<DefaultKey, Box<dyn View>>,
        size: PhysicalSize<u32>,
        shared_wgpu_state: Arc<SharedWgpuState>,
    ) -> Self {
        let device = &shared_wgpu_state.device;

        let shader = device.create_shader_module(wgpu::include_wgsl!("scene_view.wgsl"));

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let views = views
            .into_iter()
            .map(|(key, view)| {
                (
                    key,
                    SceneSubView::new(view, size, &bind_group_layout, shared_wgpu_state.clone()),
                )
            })
            .collect::<HashMap<_, _>>();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
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
                cull_mode: None,
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

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDEXES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            size,
            scene,
            views,
            index_buffer,
            render_pipeline,
            _shared_wgpu_state: shared_wgpu_state,
        }
    }
}

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

const SET_TEX_COORDS: &[Vertex; 4] = &[
    Vertex {
        position: [0.0; 2],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [0.0; 2],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [0.0; 2],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [0.0; 2],
        tex_coords: [1.0, 0.0],
    },
];

const INDEXES: &[u16] = &[0, 1, 2, 1, 3, 2];