use wgpu::Device;
use wgpu::util::DeviceExt;
use winit::window::Window;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    keyboard::NamedKey,
    window::WindowBuilder,
};

use crate::scene::Scene;
use crate::view::View;

struct SceneRenderer {
    scene: Scene,

    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    window: Window,
}

impl SceneRenderer {
    async fn new(mut scene: Scene, window: Window) -> Self {
        let size = window.inner_size();

        scene.set_size(taffy::geometry::Size {
            width: size.width as f32,
            height: size.height as f32,
        });

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // # Safety
        //
        // window needs to be dropped after surface.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
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

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
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

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Scene Vertex Buffer"),
                contents: &[],
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let (index_buffer, num_indices) = Self::get_index_buffer_from_scene(&scene, &device);

        Self {
            scene,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            window,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self) {
        let new_size = self.window.inner_size();
        
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        
        // set the surface size
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        
        // reflow the scene
        let scale_factor_inv = (1.0 / self.window.scale_factor()) as f32; // not sure how to use this...
        // self.scene.view_tree.
        self.scene.set_size(taffy::geometry::Size {
            width: new_size.width as f32 * scale_factor_inv,
            height: new_size.height as f32 * scale_factor_inv,
        });

        self.vertex_buffer = self.get_vertex_buffer_from_scene();
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            // render_pass.draw(0..NUM_VERTICES, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn get_index_buffer_from_scene(scene: &Scene, device: &Device) -> (wgpu::Buffer, u32) {
        let mut indices = vec![];
        for i in 0..scene.views.len() as u16 {
            let mut new_indices = [i * 4; 6];
            let offsets = [0, 1, 2, 0, 2, 3];

            for i in 0..6 {
                new_indices[i] += offsets[i];
            }

            indices.push(new_indices);
        }

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Scene Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_indices = indices.len() as u32 * 6;

        (index_buffer, num_indices)
    }

    fn get_vertex_buffer_from_scene(&self) -> wgpu::Buffer {
        let mut height = 0.0f32;
        let mut vertices = vec![];
        let window_size = self.scene.get_size();
        for (size, position, _key) in self.scene.get_layout() {
            let x = position.x * 2.0 / window_size.width - 1.0;
            let y = position.y * 2.0 / window_size.height - 1.0;
            let w = size.width * 2.0 / window_size.width;
            let h = size.height * 2.0 / window_size.height;

            let (a, b, c, d) = (
                x,
                y,
                x + w,
                y + h,
            );

            vertices.push([
                Vertex {
                    position: [a, b, height],
                    color: [1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [c, b, height],
                    color: [1.0, 1.0, 1.0],
                },
                Vertex {
                    position: [c, d, height],
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [a, d, height],
                    color: [0.0, 1.0, 0.0],
                },
            ]);

            height += 1.0 / self.scene.views.len() as f32;
        }

        let vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Scene Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        vertex_buffer
    }
}



#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub async fn run(scene: Scene) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut scene_renderer = SceneRenderer::new(scene, window).await;

    event_loop
        .run(move |event, window_target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == scene_renderer.window().id() => match event {
                WindowEvent::RedrawRequested => {
                    scene_renderer.update();
                    scene_renderer.render().unwrap();
                },
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
                    scene_renderer.resize();
                    scene_renderer.window().request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {

                }
                _ => {}
            },
            _ => {}
        })
        .unwrap();
}
