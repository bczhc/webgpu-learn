use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const INPUT_SIZE: usize = 32;
const RUNS_PER_BATCH: u32 = 65536; // 每次迭代处理的任务量

fn main() {
    // 初始输入数据
    let initial_input = [0_u8; INPUT_SIZE];
    let mut current_base_input = initial_input;

    // 用于通知所有线程停止的标志
    let found = Arc::new(AtomicBool::new(false));

    // --- 新增：用于统计总哈希次数的计数器 ---
    let hash_count = Arc::new(AtomicU64::new(0));
    let hash_count_clone = Arc::clone(&hash_count);

    // --- 新增：启动监控线程打印算力 ---
    std::thread::spawn(move || {
        let mut last_count = 0;
        let mut last_time = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let current_count = hash_count_clone.load(Ordering::Relaxed);
            let now = Instant::now();

            let delta = current_count - last_count;
            let duration = now.duration_since(last_time).as_secs_f64();
            let hashrate = delta as f64 / duration;

            println!(
                "当前算力: {} H/s (总计: {})",
                (hashrate as u64).to_formatted_string(&Locale::en),
                current_count.to_formatted_string(&Locale::en)
            );

            last_count = current_count;
            last_time = now;
        }
    });

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

                // --- 新增：累加计数 ---
                hash_count.fetch_add(1, Ordering::Relaxed);

                if result[0] == 0 && result[1] == 0 && result[2] == 0 && result[3] >> 4 == 0 {
                    if !found.swap(true, Ordering::SeqCst) {
                        println!("\n找到匹配!");
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
