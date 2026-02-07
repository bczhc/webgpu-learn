use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use glam::{Mat4, Vec3, Quat};
use wgpu::TextureFormat;
use wgpu_playground::wgpu_instance_with_env_backend;

fn parse_raw_data() -> Vec<f64> {
    let text = include_str!("../../data/webgpu-bg-data.txt");
    text.lines().filter(|x| !x.is_empty()).map(|x| {
        x.trim_end_matches(',').replace(' ', "").parse::<f64>().unwrap()
    }).collect()
}

// ---------------------------------------------------------
// 顶点数据结构
// ---------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
}

// 对应 WGSL 中的 struct Uniforms
// 注意对齐：vec3f 在 uniform 中占 16 字节
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: [f32; 16],
    view_position: [f32; 3],
    _pad1: f32, // 补齐到 16 字节
    light_position: [f32; 3],
    shininess: f32, // 刚好补位
}

// 对应 WGSL 中的 struct Inst
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    matrix: [f32; 16],
}

// ---------------------------------------------------------
// 渲染状态
// ---------------------------------------------------------
struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,

    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,

    uniform_buffer: wgpu::Buffer,
    storage_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,

    msaa_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,

    start_time: Instant,
    instances: Vec<InstanceInfo>,
    surface_format: TextureFormat,
}

struct InstanceInfo {
    offset: Vec3,
    time_offset: f32,
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu_instance_with_env_backend();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: Default::default(),
        }).await.unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // --- Shader ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/webgpu-bg.wgsl").into()),
        });

        // --- Data Setup ---
        let raw_data = parse_raw_data().iter().map(|&x| x as f32).collect::<Vec<_>>();
        let raw_data: &[f32] = &raw_data;
        let num_vertices = (raw_data.len() / (3 + 3 + 4)) as u32;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(raw_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // --- Instances ---
        let num_instances = 1000;
        let g_angle = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
        let mut instances = Vec::with_capacity(num_instances);
        let mut initial_matrices = Vec::with_capacity(num_instances);

        for i in 0..num_instances {
            let i_f = i as f32;
            let t = i_f * g_angle;
            let r = (i_f / num_instances as f32).sqrt() * 2.0;
            let c = t.cos();
            let s = t.sin();

            instances.push(InstanceInfo {
                offset: Vec3::new(c * r, s * r, 0.0),
                time_offset: i_f / num_instances as f32,
            });
            initial_matrices.push(InstanceData { matrix: Mat4::IDENTITY.to_cols_array() });
        }

        let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Storage Buffer"),
            contents: bytemuck::cast_slice(&initial_matrices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- Pipeline ---
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: storage_buffer.as_entire_binding() },
            ],
            label: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("myVSMain"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 40, // (3+3+4)*4
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("myFSMain"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                ..Default::default()
            },
            cache: None,
            multiview_mask: None,
        });

        let (msaa_view, depth_view) = Self::create_textures(&device, &config);

        Self {
            surface, device, queue, config, size, window,
            pipeline, vertex_buffer, num_vertices,
            uniform_buffer, storage_buffer, bind_group,
            msaa_view, depth_view,
            start_time: Instant::now(),
            instances, surface_format: format
        }
    }

    fn create_textures(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> (wgpu::TextureView, wgpu::TextureView) {
        let msaa_tex = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("MSAA Texture"),
            view_formats: &[],
        });
        let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("Depth Texture"),
            view_formats: &[],
        });
        (msaa_tex.create_view(&Default::default()), depth_tex.create_view(&Default::default()))
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            let (mv, dv) = Self::create_textures(&self.device, &self.config);
            self.msaa_view = mv;
            self.depth_view = dv;
        }
    }

    fn update(&mut self) {
        let time = self.start_time.elapsed().as_secs_f32();

        // 1. 更新 Uniforms
        let aspect = self.size.width as f32 / self.size.height as f32;
        let fov_y = 30.0f32.to_radians();
        let projection = Mat4::perspective_lh(fov_y, aspect, 0.01, 50.0);

        let half_size = 1.5;
        let fov_x = 2.0 * ((fov_y * 0.5).tan() * aspect).atan();
        let dist_x = half_size / (fov_x * 0.5).tan();
        let dist_y = half_size / (fov_y * 0.5).tan();
        let eye = Vec3::new(0.0, 0.0, dist_x.min(dist_y));

        let view = Mat4::look_at_lh(eye, Vec3::ZERO, Vec3::Y);
        let view_proj = projection * view;

        let uniforms = Uniforms {
            view_projection: view_proj.to_cols_array(),
            view_position: eye.into(),
            _pad1: 0.0,
            light_position: [2.0, 3.0, 6.0],
            shininess: 150.0,
        };
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // 2. 更新 Storage Buffer (Instances)
        let mut matrix_data = Vec::with_capacity(self.instances.len());
        for info in &self.instances {
            let t = time * 0.1 + info.time_offset * std::f32::consts::PI * 2.0;
            let mut mat = Mat4::from_translation(info.offset);
            mat = mat * Mat4::from_rotation_z(t);
            mat = mat * Mat4::from_rotation_x(t * 0.9);
            mat = mat * Mat4::from_scale(Vec3::splat(3.0));
            matrix_data.push(InstanceData { matrix: mat.to_cols_array() });
        }
        self.queue.write_buffer(&self.storage_buffer, 0, bytemuck::cast_slice(&matrix_data));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_format.add_srgb_suffix()),
            ..Default::default()
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    depth_slice: None,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0.4, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..self.instances.len() as u32);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn window(&self) -> &Window { &self.window }
}

// ---------------------------------------------------------
// 应用框架 (Winit)
// ---------------------------------------------------------
#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("WebGPU to wgpu-rust")).unwrap());
        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => state.resize(physical_size),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("{:?}", e),
                }
                state.window().request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}