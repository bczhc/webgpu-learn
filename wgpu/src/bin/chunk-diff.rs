#![feature(decl_macro)]
#![feature(file_buffered)]

/// GPU takes more time for this specific memory-bound task.

use bytemuck::cast_slice;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use rand::rngs::OsRng;
use rand::TryRngCore;
use tokio::sync::oneshot;
use wgpu::wgt::PollType;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
    BufferBinding, BufferDescriptor, BufferUsages, ComputePipeline, ComputePipelineDescriptor, Device,
    Instance, MapMode, PipelineCompilationOptions, Queue,
};

macro default() {
    Default::default()
}

const WORKGROUP_SIZE: u64 = 256;
const WORK_NUM_PER_THREAD: u64 = 4;

struct State {
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    base_buffer: Buffer,
    new_buffer: Buffer,
    result_buffer: Buffer,
    bind_group: BindGroup,
    pix_buf_len: u64,
}

impl State {
    async fn new(pix_buf_len: u64) -> anyhow::Result<Self> {
        if pix_buf_len % 4 != 0 {
            return Err(anyhow::anyhow!("pix_buf_len requires a multiple of 4"));
        }
        let instance = Instance::default();
        let adapter = instance.request_adapter(&default!()).await?;
        let (device, queue) = adapter.request_device(&default!()).await?;

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/chunk-diff.wgsl"));
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader_module,
            entry_point: None,
            compilation_options: PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", WORKGROUP_SIZE as f64),
                    ("WORK_NUM_PER_THREAD", WORK_NUM_PER_THREAD as f64),
                ],
                zero_initialize_workgroup_memory: false,
            },
            cache: None,
        });

        let base_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: pix_buf_len,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let new_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: pix_buf_len,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let result_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: pix_buf_len,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &base_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &new_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Ok(Self {
            queue,
            device,
            pipeline,
            base_buffer,
            new_buffer,
            bind_group,
            result_buffer,
            pix_buf_len,
        })
    }

    fn write_pixel_buffer(&self, base_buf: &[u8], new_buf: &[u8]) {
        self.queue.write_buffer(&self.base_buffer, 0, base_buf);
        self.queue.write_buffer(&self.new_buffer, 0, new_buf);
    }

    fn work(&self) {
        let dispatch_count = (self.pix_buf_len / 4 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let dispatch_count = (dispatch_count + WORK_NUM_PER_THREAD - 1) / WORK_NUM_PER_THREAD;
        let dispatch_count: u32 = dispatch_count.try_into().unwrap();
        let mut encoder = self.device.create_command_encoder(&default!());

        let mut pass = encoder.begin_compute_pass(&default!());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, default!());
        pass.dispatch_workgroups(dispatch_count, 1, 1);
        drop(pass);

        encoder.copy_buffer_to_buffer(&self.base_buffer, 0, &self.result_buffer, 0, None);

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const PIX_DATA_LEN: usize = 1000_000 * 128;
    let mut buf1 = vec![0_u8; PIX_DATA_LEN];
    let mut buf2 = vec![0_u8; PIX_DATA_LEN];
    // let mut buf1 = read_pix_file("data/wplace/1.pix.zst");
    // let mut buf2 = read_pix_file("data/wplace/2.pix.zst");
    OsRng.try_fill_bytes(&mut buf1)?;
    OsRng.try_fill_bytes(&mut buf2)?;

    let state = State::new(PIX_DATA_LEN as _).await?;
    let mut result_diff = vec![0_u8; PIX_DATA_LEN];

    loop {
        let instant = Instant::now();
        state.write_pixel_buffer(&buf1, &buf2);
        state.work();
        state.read_result(&mut result_diff).await?;
        // assert_eq!(result_diff, diff_chunk_owned(&image1, &image2));
        // diff_chunk(&mut buf1, &buf2);
        let d = instant.elapsed();
        println!("Duration: {:?}", d);
    }

    Ok(())
}

pub const MUTATION_MASK: u8 = 0b0100_0000;
pub const PALETTE_INDEX_MASK: u8 = 0b0011_1111;

/// Diff the two buffer. New data will be written back to `base_buf`.
#[inline(always)]
pub fn diff_chunk(base_buf: &mut [u8], new_buf: &[u8]) {
    for (b, &n) in base_buf.iter_mut().zip(new_buf) {
        let i1 = *b & PALETTE_INDEX_MASK;
        let i2 = n & PALETTE_INDEX_MASK;

        let mutated = i2 | MUTATION_MASK;

        *b = if i1 == i2 { 0 } else { mutated };
    }
}

fn diff_chunk_owned(base_buf: &[u8], new_buf: &[u8]) -> Vec<u8> {
    let mut base_cloned = vec![0_u8; base_buf.len()];
    base_cloned.copy_from_slice(base_buf);
    diff_chunk(&mut base_cloned, new_buf);
    base_cloned
}

fn read_pix_file(path: impl AsRef<Path>) ->anyhow::Result<Vec<u8>> {
    Ok(zstd::decode_all(File::open_buffered(path)?)?)
}
