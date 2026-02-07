#![feature(decl_macro)]

/// TODO: WIP

use bytemuck::cast_slice;
use tokio::sync::oneshot;
use wgpu::wgt::PollType;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
    BufferBinding, BufferDescriptor, BufferUsages, ComputePipeline, ComputePipelineDescriptor, Device,
    Instance, MapMode, PipelineCompilationOptions, Queue,
};
use wgpu_playground::wgpu_instance_with_env_backend;

macro default() {
    Default::default()
}

struct State {
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    work_buffer: Buffer,
    result_buffer: Buffer,
    bind_group: BindGroup,
}

impl State {
    async fn new(work_buffer_size: u64) -> anyhow::Result<Self> {
        let instance = wgpu_instance_with_env_backend();
        let adapter = instance.request_adapter(&default!()).await?;
        let (device, queue) = adapter.request_device(&default!()).await?;

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/compute-demo.wgsl"));
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader_module,
            entry_point: None,
            compilation_options: PipelineCompilationOptions {
                constants: &[("WORKGROUP_SIZE", 256.0)],
                zero_initialize_workgroup_memory: false,
            },
            cache: None,
        });

        let work_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: work_buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let result_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: work_buffer_size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &work_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Ok(Self {
            queue,
            device,
            pipeline,
            work_buffer,
            bind_group,
            result_buffer,
        })
    }

    fn write_work_buffer(&self, buf: &[u8]) {
        self.queue.write_buffer(&self.work_buffer, 0, buf);
    }

    fn compute_dispatch(&self, workgroups: (u32, u32, u32)) {
        let mut encoder = self.device.create_command_encoder(&default!());

        let mut pass = encoder.begin_compute_pass(&default!());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, default!());
        pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        drop(pass);

        encoder.copy_buffer_to_buffer(&self.work_buffer, 0, &self.result_buffer, 0, None);

        let command_buffer = encoder.finish();
        self.queue.submit([command_buffer]);
    }

    async fn read_result(&self, to: &mut [u8]) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.result_buffer.map_async(MapMode::Read, .., |e| {
            tx.send(e).unwrap();
        });
        self.device.poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        rx.await??;

        to[..(self.result_buffer.size() as usize)]
            .copy_from_slice(cast_slice(&*self.result_buffer.get_mapped_range(..)));
        self.result_buffer.unmap();
        Ok(())
    }
}

fn main() {
    let mut points = Vec::new();
    for line in include_str!("../../data/points.txt").lines() {
        let mut split = line.split(", ");
        let x: f64 = split.next().unwrap().parse().unwrap();
        let y: f64 = split.next().unwrap().parse().unwrap();
        points.push((x, y));
    }
}
