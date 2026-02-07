use std::env;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::RenderEncoder;
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferDescriptor,
    BufferUsages, Color, ColorTargetState, Device, FragmentState, Instance, Queue,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState,
};
use winit::event::{ElementState, MouseButton};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowId},
};
use wgpu_playground::wgpu_instance_with_env_backend;

struct State {
    start: Instant,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    module: ShaderModule,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
}

#[rustfmt::skip]
static VERTICES_DATA: [f32; 15] = {
    // 使用 f32 常量计算
    const SQRT_3: f32 = 1.732050808;  // √3
    const SIDE: f32 = 1.0;
    const HALF_SIDE: f32 = SIDE / 2.0;
    const HEIGHT: f32 = SQRT_3 * HALF_SIDE;  // √3/2 * 边长

    [
        // 顶部顶点 (红色)
        0.0, HEIGHT * 2.0 / 3.0, 1.0, 0.0, 0.0,
        // 左下角顶点 (绿色)
        -HALF_SIDE, -HEIGHT / 3.0, 0.0, 1.0, 0.0,
        // 右下角顶点 (蓝色)
        HALF_SIDE, -HEIGHT / 3.0, 0.0, 0.0, 1.0,
    ]
};

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

        let shader_module = device.create_shader_module(include_wgsl!("../shaders/triangle-rotation.wgsl"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            vertex: VertexState {
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[
                    // slot 0
                    VertexBufferLayout {
                        array_stride: 5 * 4,
                        attributes: &[
                            // position 0: vertex
                            VertexAttribute {
                                format: VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 0,
                            },
                            // position 1: color
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: 2 * 4,
                                shader_location: 1,
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
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = Self::create_vertex_buffer(&device, &queue, &VERTICES_DATA);
        let buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: 1 * 4,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let state = State {
            uniform_buffer: buffer,
            start: Instant::now(),
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

    fn create_vertex_buffer(device: &Device, queue: &Queue, data: &[f32]) -> Buffer {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: data.len() as u64 * 4,
            usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(data));
        buffer
    }

    fn update_vertex_buffer_colors(&mut self) {
        let mut new_data = VERTICES_DATA;
        let strides = new_data.len() / 5;
        for i in 0..strides {
            let start = i * 5 + 2;
            new_data[start..(start + 3)].copy_from_slice(&random_color()[..]);
        }
        self.vertex_buffer.destroy();
        self.vertex_buffer = Self::create_vertex_buffer(&self.device, &self.queue, &new_data);
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

        let t = Instant::now().duration_since(self.start).as_secs_f64() as f32;
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[t]));

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(self.uniform_buffer.as_entire_buffer_binding()),
            }],
        });

        // If you wanted to call any drawing commands, they would go here.
        if t as u32 % 5 == 0 {
            self.update_vertex_buffer_colors();
        }
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..3, 0..1);

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
            WindowEvent::MouseInput {
                state: e_state,
                button,
                ..
            } => {
                if e_state == ElementState::Pressed && button == MouseButton::Left {
                    // click; update the vertex colors
                    state.update_vertex_buffer_colors();
                    state.render();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
    //
    // To change the log level, set the `RUST_LOG` environment variable. See the `env_logger`
    // documentation for more information.
    unsafe {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);

    // When the current loop iteration finishes, suspend the thread until
    // another event arrives. Helps keeping CPU utilization low if nothing
    // is happening, which is preferred if the application might be idling in
    // the background.
    // event_loop.set_control_flow(ControlFlow::Wait);

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
