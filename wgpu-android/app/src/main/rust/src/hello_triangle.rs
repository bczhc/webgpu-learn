use crate::{default, AndroidWindow};
use wgpu::{
    include_wgsl, Color, ColorTargetState, Device, FragmentState, Instance, LoadOp, Operations,
    PresentMode, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, StoreOp, Surface, SurfaceConfiguration, TextureFormat,
    TextureUsages, TextureViewDescriptor, VertexState,
};

pub struct State {
    surface: Surface<'static>,
    pipeline: RenderPipeline,
    texture_format: TextureFormat,
    device: Device,
    queue: Queue,
    size: (u32, u32),
}

impl State {
    pub async fn new(window: AndroidWindow) -> anyhow::Result<Self> {
        let instance = Instance::default();

        let window_size = (window.width, window.height);

        let adapter = instance.request_adapter(&default!()).await?;
        let (device, queue) = adapter.request_device(&default!()).await?;

        let surface = instance.create_surface(window)?;
        let cap = surface.get_capabilities(&adapter);
        let texture_format = cap.formats[0];

        let shader_module = device.create_shader_module(include_wgsl!("./hello_triangle.wgl"));
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: VertexState {
                buffers: &[],
                entry_point: None,
                module: &shader_module,
                compilation_options: default!(),
            },
            primitive: default!(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: texture_format,
                    blend: None,
                    write_mask: default!(),
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let state = Self {
            pipeline,
            texture_format,
            surface,
            device,
            queue,
            size: window_size,
        };
        state.configure_surface();
        Ok(state)
    }

    pub fn configure_surface(&self) {
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self.texture_format,
                width: self.size.0,
                height: self.size.1,
                present_mode: PresentMode::AutoVsync,
                desired_maximum_frame_latency: 2,
                alpha_mode: Default::default(),
                view_formats: vec![self.texture_format.add_srgb_suffix()],
            },
        );
    }

    fn update(&mut self) {}

    pub fn update_size(&mut self, window_size: (u32, u32)) {
        self.size = window_size;
    }

    pub fn render(&self) -> anyhow::Result<()> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture.texture.create_view(&TextureViewDescriptor {
            format: Some(self.texture_format.add_srgb_suffix()),
            ..default!()
        });

        let mut encoder = self.device.create_command_encoder(&default!());
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::WHITE),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.draw(0..3, 0..1);

        drop(pass);
        let command_buffer = encoder.finish();

        self.queue.submit([command_buffer]);
        surface_texture.present();
        Ok(())
    }
}

pub async fn show(window: AndroidWindow) -> anyhow::Result<()> {
    Ok(())
}
