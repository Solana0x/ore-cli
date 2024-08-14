use std::{collections::HashMap, env, sync::{Arc, Mutex}, time::Instant};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use rand::Rng;
use drillx::equix::SolverMemory;

fn main() {
    // 获取线程数
    let args: Vec<String> = env::args().collect();
    let threads = args.get(1).map_or(1, |t| t.parse::<usize>().unwrap_or(1));
    println!("Using {} threads", threads);

    // 初始化计时器
    let timer = Instant::now();

    // 生成随机的 challenge
    let challenge: [u8; 32] = rand::thread_rng().gen();

    // 用于存储不同难度的计数
    let hash_count = Arc::new(Mutex::new(HashMap::<u32, u64>::new()));
    let total_hashes = Arc::new(AtomicU64::new(0));

    // 设置 Rayon 使用的线程数量
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().unwrap();

    // 并行处理 nonce 值
    (0..threads).into_par_iter().for_each(|i| {
        let mut memory = SolverMemory::new();
        let first_nonce = u64::MAX / threads as u64 * i as u64;
        let mut nonce = first_nonce;
        let local_hash_count = Arc::clone(&hash_count);
        let local_total_hashes = Arc::clone(&total_hashes);
        let local_timer = Instant::now();

        loop {
            // 计算哈希值
            if let Ok(hx) = drillx::hash_with_memory(
                &mut memory,&challenge, &nonce.to_le_bytes()) {
                let diff = hx.difficulty();

                // 更新总哈希数
                local_total_hashes.fetch_add(1, Ordering::Relaxed);

                // 更新不同难度的哈希计数
                {
                    let mut hash_count = local_hash_count.lock().unwrap();
                    *hash_count.entry(diff).or_insert(0) += 1;
                }

                // 每 100 个 nonce 打印一次状态
                if nonce % 1000 == 0 {
                    print(&*local_hash_count.lock().unwrap(), local_total_hashes.load(Ordering::Relaxed), &timer);
                }
            }

            // 增加 nonce 值
            nonce += 1;

            // 如果时间超过设定的测试时长，跳出循环
            if (local_timer.elapsed().as_secs() as i64).ge(&55) {
                break;
            }
        }
    });

    // 打印最终结果
    print(&*hash_count.lock().unwrap(), total_hashes.load(Ordering::Relaxed), &timer);
}

fn print(hash_counts: &HashMap<u32, u64>, total_hashes: u64, timer: &Instant) {
    let max_key = *hash_counts.keys().max().unwrap_or(&0);
    let elapsed_time = timer.elapsed().as_secs();
    let average_hashes_per_sec = if elapsed_time > 0 {
        total_hashes / elapsed_time
    } else {
        0
    };

    let mut str = format!("{} sec – Avg Hashes/Sec: {} – ", elapsed_time, average_hashes_per_sec);
    for i in 0..=max_key {
        str = format!("{} {}: {} ", str, i, hash_counts.get(&i).unwrap_or(&0)).to_string();
    }
    println!("{}", str);
}
