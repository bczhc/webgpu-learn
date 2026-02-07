use std::env;
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wgpu::util::RenderEncoder;
use wgpu::VertexFormat::Float32x2;
use wgpu::{
    include_wgsl, Buffer, BufferDescriptor, BufferUsages, Color, ColorTargetState, Device,
    FragmentState, Instance, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, VertexAttribute, VertexBufferLayout, VertexState,
};
use winit::event::{ElementState, MouseButton};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowId},
};
use winit::keyboard::{Key, NamedKey, PhysicalKey};
use wgpu_playground::wgpu_instance_with_env_backend;

struct State {
    elapsed: PausableTimeElapse,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    module: ShaderModule,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

impl State {
    async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> State {
        // let instance = wgpu::Instance::new(
        //     wgpu::InstanceDescriptor::default().with_display_handle(Box::new(display)),
        // );
        let instance = wgpu_instance_with_env_backend();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);

        let surface_format = cap.formats[0];

        let shader_module = device.create_shader_module(include_wgsl!("../shaders/lissajous.wgsl"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            vertex: VertexState {
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[
                    // slot 0
                    VertexBufferLayout {
                        array_stride: 2 * 4,
                        attributes: &[
                            // position 0
                            VertexAttribute {
                                format: Float32x2,
                                offset: 0,
                                shader_location: 0,
                            },
                        ],
                        step_mode: Default::default(),
                    },
                ],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: None,
                    write_mask: Default::default(),
                })],
            }),
            label: None,
            layout: None,
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = Self::create_vertex_buffer(&device, 65536 * 4);
        let state = State {
            elapsed: PausableTimeElapse::new(),
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            module: shader_module,
            pipeline,
            vertex_buffer,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn create_vertex_buffer(device: &Device, size: u64) -> Buffer {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        buffer
    }

    fn get_window(&self) -> &Window {
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
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self) {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Renders a gray screen
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Color::from_vec4d([0.3, 0.3, 0.3, 1.0])),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let elapsed = self.elapsed.elapsed().as_secs_f64() as f32;

        const SEGMENTS: usize = 2000;
        let mut buf = [0f32; SEGMENTS * 2];
        for (i, x) in (0..=(SEGMENTS - 1)).into_iter().enumerate() {
            let t = (x as f32 / (SEGMENTS - 1) as f32) * 2.0 * PI;
            let x = 0.5 * (elapsed / 2.0 * t + elapsed).sin();
            let y = 0.5 * (2.0 * t * elapsed).sin();
            buf[i * 2] = x;
            buf[i * 2 + 1] = y;
        }
        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&buf),
        );
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..(SEGMENTS as u32), 0..1);

        // End the renderpass.
        drop(pass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                // Emits a new redraw requested event.
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
            }
            WindowEvent::KeyboardInput {event, ..} => {
                if event.logical_key == Key::Named(NamedKey::Space) && event.state == ElementState::Pressed {
                    // switch paused state
                    state.elapsed.switch_pause();
                }
            }
            WindowEvent::MouseInput {
                state: e_state,
                button,
                ..
            } => {
                if e_state == ElementState::Pressed && button == MouseButton::Left {
                    // click; update the vertex colors
                    state.render();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    unsafe {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}

trait ColorExt {
    fn from_vec4d(x: [f64; 4]) -> Self;
}

impl ColorExt for Color {
    fn from_vec4d(x: [f64; 4]) -> Self {
        Self {
            r: x[0],
            g: x[1],
            b: x[2],
            a: x[3],
        }
    }
}

fn random_color() -> [f32; 3] {
    [
        rand::random::<f32>(),
        rand::random::<f32>(),
        rand::random::<f32>(),
    ]
}

struct PausableTimeElapse {
    start: Option<Instant>,
    elapsed: Duration,
}

impl PausableTimeElapse {
    fn new() -> Self {
        Self {
            start: Some(Instant::now()),
            elapsed: Duration::ZERO,
        }
    }

    fn elapsed(&self) -> Duration {
        match self.start {
            Some(t) => self.elapsed + t.elapsed(),
            None => self.elapsed,
        }
    }

    fn switch_pause(&mut self) {
        match self.start {
            Some(t) => {
                self.elapsed += t.elapsed();
                self.start = None;
            }
            None => {
                self.start = Some(Instant::now());
            }
        }
    }
}
