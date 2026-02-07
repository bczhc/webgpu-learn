/// Native wgpu version of https://cznull.github.io/vsbm
///
/// Co-worked with Gemini.

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, iter};
use wgpu::{Backend, Backends, InstanceDescriptor};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};
use wgpu_playground::wgpu_instance_with_env_backend;

// --- Uniform 数据结构 (必须符合 WGSL 的 16 字节对齐) ---
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    origin: [f32; 3],
    padding1: f32,
    right: [f32; 3],
    padding2: f32,
    up: [f32; 3],
    padding3: f32,
    forward: [f32; 3],
    padding4: f32,
    screen_size: [f32; 2],
    len: f32,
    padding5: f32,
}

struct FpsCounter {
    instant: Instant,
    counter: usize,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            instant: Instant::now(),
            counter: 0,
        }
    }

    fn hint_and_get(&mut self) -> (Duration, f32) {
        self.counter += 1;
        let duration = self.instant.elapsed();
        (duration, (self.counter as f64 / duration.as_secs_f64()) as f32)
    }
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    start_time: Instant,
    window: Arc<Window>,
    surface_format: wgpu::TextureFormat,
    fps_counter: Option<FpsCounter>,
}

impl State {
    fn window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoNoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> Self {
        let saved_window = Arc::clone(&window);
        let size = window.inner_size();
        let instance = wgpu_instance_with_env_backend();
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 0,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format.add_srgb_suffix()],
        };
        surface.configure(&device, &config);

        // --- 核心 WGSL 着色器 ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/vsbm.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format.add_srgb_suffix(),
                    blend: None,
                    write_mask: Default::default(),
                })],
            }),
            multiview_mask: None,
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            size,
            render_pipeline,
            uniform_buffer,
            uniform_bind_group,
            start_time: Instant::now(),
            window: saved_window,
            surface_format,
            fps_counter: None,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn update(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let ang1 = 2.8 + elapsed * 0.5; // 自动旋转
        let ang2: f32 = 0.4;
        let len = 1.6;

        let origin = [
            len * ang1.cos() * ang2.cos(),
            len * ang2.sin(),
            len * ang1.sin() * ang2.cos(),
        ];
        let right = [ang1.sin(), 0.0, -ang1.cos()];
        let up = [
            -ang2.sin() * ang1.cos(),
            ang2.cos(),
            -ang2.sin() * ang1.sin(),
        ];
        let forward = [
            -ang1.cos() * ang2.cos(),
            -ang2.sin(),
            -ang1.sin() * ang2.cos(),
        ];

        let cx = self.size.width as f32;
        let cy = self.size.height as f32;
        let sx = (cx.min(cy) / cx) * (cx / cx.max(cy));
        let sy = (cy.min(cx) / cy) * (cy / cx.max(cy));

        // 因为使用了 1:1 的 Viewport，这里 screen_size 直接给 1.0 即可
        let uniforms = Uniforms {
            origin,
            padding1: 0.0,
            right,
            padding2: 0.0,
            up,
            padding3: 0.0,
            forward,
            padding4: 0.0,
            screen_size: [1.0, 1.0],
            len,
            padding5: 0.0,
        };

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.07,
                            g: 0.06,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            // --- 核心逻辑：设置居中的 1:1 正方形视口 ---
            let win_w = self.size.width as f32;
            let win_h = self.size.height as f32;
            let side = win_w.min(win_h); // 取短边
            let x_offset = (win_w - side) / 2.0;
            let y_offset = (win_h - side) / 2.0;

            render_pass.set_viewport(x_offset, y_offset, side, side, 0.0, 1.0);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        surface_texture.present();

        // calculate the FPS
        if let Some(f) = &mut self.fps_counter {
            let (d, fps) = f.hint_and_get();
            if d.as_secs_f64() > 1.0 {
                println!("FPS: {}", fps);
                self.fps_counter = Some(FpsCounter::new());
            }
        } else {
            self.fps_counter = Some(FpsCounter::new());
        }
        Ok(())
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(State::new(
            event_loop.owned_display_handle(),
            window.clone(),
        ));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
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

pub fn main() {
    unsafe {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
