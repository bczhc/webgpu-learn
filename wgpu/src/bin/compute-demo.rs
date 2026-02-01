#![feature(decl_macro)]

use bytemuck::cast_slice;
use bytemuck::checked::cast_slice_mut;
use rand::Rng;
use std::time::Instant;
use tokio::sync::oneshot;
use wgpu::wgt::PollType;
use wgpu::{include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferDescriptor, BufferUsages, ComputePipeline, ComputePipelineDescriptor, Device, Instance, MapMode, PipelineCompilationOptions, Queue};
use wgpu_playground::set_up_logger;

macro default() {
    Default::default()
}

const WORKGROUP_SIZE: usize = 256;

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
        let instance = Instance::default();
        let adapter = instance.request_adapter(&default!()).await?;
        let (device, queue) = adapter.request_device(&default!()).await?;

        let shader_module = device.create_shader_module(include_wgsl!("../shaders/compute-demo.wgsl"));
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader_module,
            entry_point: None,
            compilation_options: PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", WORKGROUP_SIZE as f64)
                ],
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

async fn boring_stress_test() -> anyhow::Result<()> {
    const DATA_LENGTH: usize = 100_000;
    const WORKGROUP_COUNT: usize = (DATA_LENGTH + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE; /* =ceil(DATA_LENGTH / WORKGROUP_SIZE) */
    let mut input = vec![0_f32; DATA_LENGTH];
    let mut result = vec![0_f32; DATA_LENGTH];
    let state = State::new(input.len() as u64 * 4).await?;

    let mut rng = rand::rng();
    loop {
        let instant = Instant::now();
        input.iter_mut().for_each(|x| *x = rng.random());
        state.write_work_buffer(cast_slice(&input));
        state.compute_dispatch((WORKGROUP_COUNT as u32, 1, 1));
        state.read_result(cast_slice_mut(&mut result)).await?;
        let duration = instant.elapsed();
        // println!("Input: {:?}, Output: {:?}", &input[..5], &result[..5]);
        println!("Duration: {:?}", duration);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    set_up_logger();

    boring_stress_test().await?;

    Ok(())
}
