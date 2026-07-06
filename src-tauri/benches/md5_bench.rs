use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::time::Instant;

const BUFFER_SIZE: usize = 64 * 1024;

fn file_md5(path: &str) -> String {
    let file = File::open(path).expect("open");
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut context = md5::Context::new();
    loop {
        let n = reader.read(&mut buffer).expect("read");
        if n == 0 {
            break;
        }
        context.consume(&buffer[..n]);
    }
    format!("{:x}", context.finalize())
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join("md5bench_104857600.bin")
        });
    let meta = std::fs::metadata(&path).expect("metadata");
    let size_mb = meta.len() as f64 / (1024.0 * 1024.0);
    println!("File: {} ({:.1} MB)", path, size_mb);

  // warmup
    file_md5(path.to_str().unwrap());

    let start = Instant::now();
    let hash = file_md5(path.to_str().unwrap());
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let throughput = meta.len() as f64 / secs / (1024.0 * 1024.0);
    println!("MD5: {} ...{}", hash.chars().take(8).collect::<String>(), hash.chars().skip(24).collect::<String>());
    println!("Time: {:.2}s, throughput: {:.1} MB/s", secs, throughput);
    let est_2g = 2048.0 / throughput;
    println!("Estimated 2GB: {:.1}s ({:.1} min)", est_2g, est_2g / 60.0);
}
