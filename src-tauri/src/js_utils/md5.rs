use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::preview_session::PreviewSession;

/// 缓冲区大小：64KB，适合大文件流式处理
const BUFFER_SIZE: usize = 64 * 1024;

/// 预览模式下，超过此文件数量时跳过 MD5 计算
pub const PREVIEW_FILE_COUNT_THRESHOLD: usize = 20;

/// 预览模式下，超过此大小的文件跳过 MD5 计算（5MB）
pub const PREVIEW_FILE_SIZE_THRESHOLD: u64 = 5 * 1024 * 1024;

/// 预览模式下 MD5 占位符
pub const PREVIEW_PLACEHOLDER: &str = "[md5-preview]";

/// 预览时 MD5 计算策略
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Md5PreviewMode {
    /// 文件较多或较大时跳过
    #[default]
    Auto,
    /// 始终计算真实 MD5
    Always,
    /// 始终使用占位符
    Skip,
}

impl Md5PreviewMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "always" => Self::Always,
            "skip" => Self::Skip,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Default)]
pub struct Md5Options {
    pub preview_mode: bool,
    pub total_file_count: usize,
    pub preview_md5_mode: Md5PreviewMode,
    pub session: Option<PreviewSession>,
}

impl Md5Options {
    pub fn should_use_placeholder(&self, file_path: &str) -> bool {
        if !self.preview_mode {
            return false;
        }

        match self.preview_md5_mode {
            Md5PreviewMode::Always => false,
            Md5PreviewMode::Skip => true,
            Md5PreviewMode::Auto => {
                if self.total_file_count > PREVIEW_FILE_COUNT_THRESHOLD {
                    return true;
                }

                let path = Path::new(file_path);
                if let Ok(metadata) = std::fs::metadata(path) {
                    if metadata.is_file() && metadata.len() > PREVIEW_FILE_SIZE_THRESHOLD {
                        return true;
                    }
                }

                false
            }
        }
    }
}

/// 根据选项计算文件 MD5，或在预览模式下返回占位符
pub fn file_md5_with_options(file_path: String, options: &Md5Options) -> String {
    if let Some(session) = &options.session {
        if session.is_cancelled() {
            return "ERROR: 预览已取消".to_string();
        }
    }

    let result = if options.should_use_placeholder(&file_path) {
        PREVIEW_PLACEHOLDER.to_string()
    } else {
        file_md5_internal(&file_path, options.session.as_ref())
            .unwrap_or_else(|e| format!("ERROR: {}", e))
    };

    if let Some(session) = &options.session {
        session.on_file_processed(&file_path, "md5");
    }

    result
}

fn file_md5_internal(file_path: &str, session: Option<&PreviewSession>) -> Result<String> {
    let path = Path::new(file_path);

    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", file_path);
    }

    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", file_path);
    }

    let file = File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;

    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut context = md5::Context::new();

    loop {
        if session.is_some_and(|s| s.is_cancelled()) {
            anyhow::bail!("预览已取消");
        }

        let bytes_read = reader
            .read(&mut buffer)
            .with_context(|| format!("Failed to read file: {}", file_path))?;

        if bytes_read == 0 {
            break;
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
