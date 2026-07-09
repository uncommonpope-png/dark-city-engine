use crate::shader::ShaderManager;
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::asset::generate_city_vertices;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: glam::Mat4,
}

pub struct WgpuRenderer {
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    pipeline: Option<wgpu::RenderPipeline>,
    shader_manager: Option<ShaderManager>,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_count: Option<u32>,
    camera: Option<Camera>,
    camera_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
}

impl Default for WgpuRenderer {
    fn default() -> Self {
        Self {
            surface: None,
            device: None,
            queue: None,
            config: None,
            pipeline: None,
            shader_manager: None,
            vertex_buffer: None,
            vertex_count: None,
            camera: None,
            camera_buffer: None,
            bind_group: None,
        }
    }
}

impl WgpuRenderer {
    pub async fn new(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                flags: wgpu::InstanceFlags::empty(),
                dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
                gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
            });

            let width = canvas.width().max(1);
            let height = canvas.height().max(1);

            let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).map_err(|e| format!("failed to create surface: {e}"))?;

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .ok_or_else(|| "no suitable GPU adapter found".to_string())?;

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("dark_city_engine_device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults(),
                        memory_hints: wgpu::MemoryHints::Performance,
                    },
                    None,
                )
                .await
                .map_err(|err| format!("failed to request device: {err}"))?;

            let capabilities = surface.get_capabilities(&adapter);
            let format = capabilities
                .formats
                .iter()
                .copied()
                .find(|format| matches!(format, wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb))
                .unwrap_or_else(|| capabilities.formats[0]);

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);

            let mut camera = Camera::new(width, height);

            let camera_uniform = CameraUniform {
                view_proj: camera.build_view_projection(),
            };
            let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("dark_city_engine_camera_buffer"),
                contents: bytemuck::bytes_of(&camera_uniform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("dark_city_engine_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("dark_city_engine_bind_group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
            });

            let shader_manager = ShaderManager::from_files(
                "shaders/basic.vert.wgsl",
                "shaders/basic.frag.wgsl",
            )
            .await
            .map_err(|err| format!("failed to load shaders: {err}"))?;

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("dark_city_engine_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_manager.vertex_source().to_string().into()),
            });

            let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("dark_city_engine_fragment_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_manager.fragment_source().to_string().into()),
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("dark_city_engine_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let (city_positions, city_colors) = generate_city_vertices();

            let vertex_data: Vec<Vertex> = city_positions.iter().zip(city_colors.iter()).map(|(p, c)| Vertex {
                position: *p,
                color: *c,
            }).collect();

            let vertex_count = vertex_data.len();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("dark_city_engine_vertex_buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("dark_city_engine_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[Vertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
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
                cache: None,
            });

            Ok(Self {
                surface: Some(surface),
                device: Some(device),
                queue: Some(queue),
                config: Some(config),
                pipeline: Some(pipeline),
                shader_manager: Some(shader_manager),
                vertex_buffer: Some(vertex_buffer),
                vertex_count: Some(vertex_count as u32),
                camera: Some(camera),
                camera_buffer: Some(camera_buffer),
                bind_group: Some(bind_group),
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(Self::default())
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let Some(config) = self.config.as_mut() else {
            return;
        };

        config.width = width.max(1);
        config.height = height.max(1);

        if let Some(device) = self.device.as_ref() {
            if let Some(surface) = self.surface.as_ref() {
                surface.configure(device, config);
            }
        }

        if let Some(camera) = self.camera.as_mut() {
            camera.resize(width, height);
            self.update_camera_buffer();
        }
    }

    pub fn update_camera_buffer(&self) {
        if let (Some(device), Some(queue), Some(camera_buffer), Some(camera)) =
            (self.device.as_ref(), self.queue.as_ref(), self.camera_buffer.as_ref(), self.camera.as_ref())
        {
            let uniform = CameraUniform {
                view_proj: camera.build_view_projection(),
            };
            queue.write_buffer(camera_buffer, 0, bytemuck::bytes_of(&uniform));
        }
    }

    pub fn camera_mut(&mut self) -> Option<&mut Camera> {
        self.camera.as_mut()
    }

    pub fn render(&mut self) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(surface) = self.surface.as_ref() else {
                return Err("renderer surface is not initialized".to_string());
            };
            let Some(device) = self.device.as_ref() else {
                return Err("renderer device is not initialized".to_string());
            };
            let Some(queue) = self.queue.as_ref() else {
                return Err("renderer queue is not initialized".to_string());
            };
            let Some(config) = self.config.as_ref() else {
                return Err("renderer config is not initialized".to_string());
            };
            let Some(pipeline) = self.pipeline.as_ref() else {
                return Err("renderer pipeline is not initialized".to_string());
            };
            let Some(vertex_buffer) = self.vertex_buffer.as_ref() else {
                return Err("renderer vertex buffer is not initialized".to_string());
            };

            let Some(bind_group) = self.bind_group.as_ref() else {
                return Err("renderer bind group is not initialized".to_string());
            };

            let Some(vertex_count) = self.vertex_count else {
                return Err("renderer vertex count is not set".to_string());
            };

            let output = surface
                .get_current_texture()
                .map_err(|err| format!("failed to acquire swap-chain texture: {err}"))?;
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("dark_city_engine_encoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("dark_city_engine_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.15,
                                g: 0.08,
                                b: 0.20,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw(0..vertex_count, 0..1);
            }

            queue.submit(std::iter::once(encoder.finish()));
            output.present();

            if config.width == 0 || config.height == 0 {
                return Err("renderer dimensions are invalid".to_string());
            }

            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(())
        }
    }
}