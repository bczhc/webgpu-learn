#![feature(decl_macro)]

use bytemuck::{cast_slice, cast_slice_mut};
use std::process::exit;
use std::time::Instant;
use tokio::sync::oneshot;
use wgpu::wgt::PollType;
use wgpu::{
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
    BufferBinding, BufferDescriptor, BufferUsages, ComputePipeline, ComputePipelineDescriptor,
    Device, Instance, InstanceDescriptor, MapMode, PipelineCompilationOptions, Queue,
    ShaderModuleDescriptor, ShaderSource,
};
use wgpu_playground::{default, wgpu_instance_with_env_backend, set_up_logger};

/// Sha256 buffer type the shader uses.
type FatSha256Buf = [u32; SHA256_BYTES];

const SHA256_BYTES: usize = 32;
const INPUT_SIZE: usize = 32;
/// The shader treats `u32`s as `u8`s.
const BLOCK_BUFFER_IN_SHADER: u64 = size_of::<FatSha256Buf>() as _;

use clap::Parser;
use sha2::Digest;

#[derive(Parser, Debug)]
#[command(about = "GPU Sha256 Miner Simulator")]
struct Args {
    /// Number of threads per workgroup (WORKGROUP_SIZE)
    #[arg(long, default_value_t = 256)]
    workgroup_size: u32,

    /// Number of workgroups to dispatch in the X dimension (DISPATCH_X)
    #[arg(long, default_value_t = 2048)]
    dispatch_x: u32,

    /// Number of hash iterations performed by each individual thread
    #[arg(short, long, default_value_t = 256)]
    iterations: u32,

    /// Target difficulty in bits
    #[arg(short, long, default_value_t = 32)]
    difficulty: u32,
}

struct State {
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    input_buffer: Buffer,
    result_buffer: Buffer,
    map_read_buffer: Buffer,
    bind_group: BindGroup,
}

impl State {
    async fn new(args: &Args) -> anyhow::Result<Self> {
        let instance = wgpu_instance_with_env_backend();
        let adapter = instance.request_adapter(&default!()).await?;
        let (device, queue) = adapter.request_device(&default!()).await?;

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(wgsl_source(args.difficulty).into()),
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader_module,
            entry_point: None,
            compilation_options: PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", args.workgroup_size as f64),
                    ("ITERATIONS_PER_THREAD", args.iterations as f64),
                    (
                        "RUNS_PER_DISPATCH",
                        (args.dispatch_x * args.workgroup_size) as f64,
                    ),
                    ("DIFFICULTY_BITS", args.difficulty as f64),
                ],
                zero_initialize_workgroup_memory: false,
            },
            cache: None,
        });

        let input_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: INPUT_SIZE as u64 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let result_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: BLOCK_BUFFER_IN_SHADER,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let map_read_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: BLOCK_BUFFER_IN_SHADER,
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
                        buffer: &input_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &result_buffer,
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
            input_buffer,
            bind_group,
            result_buffer,
            map_read_buffer,
        })
    }

    fn write_input_data(&self, buf: &[u8]) {
        let mut input_data = [0_u32; INPUT_SIZE];
        for (i, &b) in buf.iter().enumerate() {
            input_data[i] = b as _;
        }
        self.queue
            .write_buffer(&self.input_buffer, 0, cast_slice(&input_data));
    }

    fn compute_dispatch(&self, workgroups_x: u32) {
        let mut encoder = self.device.create_command_encoder(&default!());

        let mut pass = encoder.begin_compute_pass(&default!());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, default!());
        pass.dispatch_workgroups(workgroups_x, 1, 1);
        drop(pass);

        encoder.copy_buffer_to_buffer(&self.result_buffer, 0, &self.map_read_buffer, 0, None);

        let command_buffer = encoder.finish();
        self.queue.submit([command_buffer]);
    }

    async fn read_result(&self, to: &mut [u8]) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.map_read_buffer.map_async(MapMode::Read, .., |e| {
            tx.send(e).unwrap();
        });
        self.device.poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        rx.await??;

        to[..(self.map_read_buffer.size() as usize)]
            .copy_from_slice(cast_slice(&*self.map_read_buffer.get_mapped_range(..)));
        self.map_read_buffer.unmap();
        Ok(())
    }
}

fn add_big_int(data: &mut [u8; 32], mut n: u32) {
    let mut carry = n;

    for byte in data.iter_mut() {
        if carry == 0 {
            break;
        }

        // 将当前字节与进位相加
        // 先转为 u32 避免计算过程中溢出
        let sum = *byte as u32 + carry;

        // 取低 8 位存回
        *byte = (sum & 0xFF) as u8;

        // 计算新的进位
        carry = sum >> 8;
    }
}

#[inline(always)]
fn convert_fat_buf(buf: &FatSha256Buf) -> [u8; SHA256_BYTES] {
    buf.map(|x| x as u8)
}

fn generate_check_difficulty_wgsl(difficulty_bits: u32) -> String {
    let mut conditions = Vec::new();

    // 处理完整的字节 (8 bits 每组)
    let full_bytes = difficulty_bits / 8;
    for i in 0..full_bytes {
        conditions.push(format!("buf[{}] == 0u", i));
    }

    // 处理剩余的位 (非 8 整除的部分)
    let remaining_bits = difficulty_bits % 8;
    if remaining_bits > 0 {
        let shift = 8 - remaining_bits;
        // 使用索引 full_bytes 指向下一个字节
        conditions.push(format!("(buf[{}] >> {}u) == 0u", full_bytes, shift));
    }

    // 处理难度为 0 的特殊情况
    let final_condition = if conditions.is_empty() {
        "true".to_string()
    } else {
        conditions.join(" && ")
    };

    format!(
        r#"
fn check_difficulty(buf: ptr<function, array<u32, SHA256_BLOCK_SIZE>>) -> bool {{
    return {};
}}
"#,
        final_condition
    )
}

fn wgsl_source(difficulty_bits: u32) -> String {
    let mut source = include_str!("../shaders/sha256-miner.wgsl")
        .lines()
        .collect::<Vec<_>>();
    source.remove(0);
    let generated = generate_check_difficulty_wgsl(difficulty_bits);
    for x in generated.lines().into_iter().rev() {
        source.insert(0, x);
    }
    source.join("\n")
}

fn print_result_and_exit(buf: FatSha256Buf, start: Instant) {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(convert_fat_buf(&buf));
    let hash = hex::encode(hasher.finalize());

    println!("Result:");
    println!("  input: {}", hex::encode(convert_fat_buf(&buf)));
    println!("  sha256: {}", hash);
    println!("  elapsed: {:?}", start.elapsed());
    exit(0);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    set_up_logger();

    let args = Args::parse();
    let runs_per_dispatch = args.dispatch_x * args.workgroup_size;

    println!("Args: {:?}", args);

    let state = State::new(&args).await?;
    let mut input_data = [0_u8; INPUT_SIZE];
    let mut result = [0_u32; SHA256_BYTES];
    let mut counter = 0_usize;
    let start = Instant::now();
    loop {
        println!("dispatch: {}, start: {}", counter, hex::encode(input_data));
        state.write_input_data(&input_data);
        state.compute_dispatch(args.dispatch_x);
        add_big_int(&mut input_data, runs_per_dispatch * args.iterations);
        state.read_result(cast_slice_mut(&mut result)).await?;
        if result.iter().any(|x| *x != 0) {
            print_result_and_exit(result, start);
        }
        counter += 1;
    }
}

mod cpu_impl_by_gemini {
    use rayon::prelude::*;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    const INPUT_SIZE: usize = 32;
    const RUNS_PER_BATCH: u32 = 65536; // 每次迭代处理的任务量

    /// 模拟你 GPU 中的 add_big_int 逻辑
    fn add_big_int(data: &mut [u8; 32], n: u32) {
        let mut carry = n;
        for byte in data.iter_mut() {
            if carry == 0 {
                break;
            }
            let sum = *byte as u32 + carry;
            *byte = (sum & 0xFF) as u8;
            carry = sum >> 8;
        }
    }

    fn main() {
        // 初始输入数据
        let initial_input = [0_u8; INPUT_SIZE];
        let mut current_base_input = initial_input;

        // 用于通知所有线程停止的标志
        let found = Arc::new(AtomicBool::new(false));

        println!("开始在 16 线程上进行 SHA256 搜索...");

        // 配置线程池为 16 线程
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(16)
            .build()
            .unwrap();

        pool.install(|| {
            loop {
                // 并行处理当前批次
                (0..RUNS_PER_BATCH).into_par_iter().for_each(|offset| {
                    if found.load(Ordering::Relaxed) {
                        return;
                    }

                    // 准备当前线程的输入
                    let mut local_input = current_base_input;
                    add_big_int(&mut local_input, offset);

                    // 计算 SHA256
                    let mut hasher = Sha256::new();
                    hasher.update(&local_input);
                    let result = hasher.finalize();

                    if result[0] == 0 && result[1] == 0 && result[2] == 0 {
                        if !found.swap(true, Ordering::SeqCst) {
                            println!("找到匹配!");
                            println!("输入 (hex): {}", hex::encode(local_input));
                            println!("哈希 (hex): {}", hex::encode(result));
                            std::process::exit(0);
                        }
                    }
                });

                // 更新基础输入，进入下一批次
                add_big_int(&mut current_base_input, RUNS_PER_BATCH);
            }
        });
    }

    #[test]
    fn run() {
        main();
    }
}
