use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;

/// 缓冲区大小：64KB，适合大文件流式处理
const BUFFER_SIZE: usize = 64 * 1024;

/// 计算文件的 MD5 值，返回十六进制字符串
/// 使用流式处理，支持大文件而不会占用过多内存
/// 注意：只支持文件，不支持文件夹
pub fn file_md5(file_path: String) -> Result<String> {
    let path = Path::new(&file_path);
    
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", file_path);
    }
    
    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", file_path);
    }
    
    let file = File::open(&file_path)
        .with_context(|| format!("Failed to open file: {}", file_path))?;
    
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut context = md5::Context::new();
    
    loop {
        let bytes_read = reader.read(&mut buffer)
            .with_context(|| format!("Failed to read file: {}", file_path))?;
        
        if bytes_read == 0 {
            break; // 文件读取完毕
        }
        
        context.consume(&buffer[..bytes_read]);
    }
    
    let digest = context.finalize();
    Ok(format!("{:x}", digest))
}

/// 计算字符串的 MD5 值，返回十六进制字符串
pub fn string_md5(content: String) -> String {
    let digest = md5::compute(content.as_bytes());
    format!("{:x}", digest)
}
