use std::path::{Path, PathBuf};

/// 路径拼接，将多个路径段连接起来
/// 支持多个参数：join(path1, path2, ...)
/// 注意：rquickjs 会将多个参数作为元组传递，我们需要手动处理
pub fn join(segments: Vec<String>) -> String {
    if segments.is_empty() {
        return String::new();
    }

    let mut path_buf = PathBuf::new();
    for segment in segments {
        if !segment.is_empty() {
            path_buf.push(segment);
        }
    }

    path_buf.to_string_lossy().to_string()
}

/// 路径拼接的包装函数，接受两个参数（最常用的情况）
pub fn join2(path1: String, path2: String) -> String {
    join(vec![path1, path2])
}

/// 获取路径的目录部分
pub fn dirname(path: String) -> String {
    let path = Path::new(&path);
    match path.parent() {
        Some(parent) => parent.to_string_lossy().to_string(),
        None => String::new(),
    }
}

/// 获取路径的文件名部分（不包含扩展名）
pub fn basename(path: String) -> String {
    let path = Path::new(&path);
    match path.file_stem() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            // 如果没有文件名，尝试返回整个路径的最后一部分
            match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => String::new(),
            }
        }
    }
}

/// 获取路径的扩展名（包含点号）
pub fn extname(path: String) -> String {
    let path = Path::new(&path);
    match path.extension() {
        Some(ext) => format!(".{}", ext.to_string_lossy()),
        None => String::new(),
    }
}

/// 规范化路径，处理 `..` 和 `.`
pub fn normalize(path: String) -> String {
    let path_buf = PathBuf::from(&path);
    path_buf.canonicalize()
        .unwrap_or(path_buf)
        .to_string_lossy()
        .to_string()
}

/// 判断路径是否为绝对路径
pub fn is_absolute(path: String) -> bool {
    Path::new(&path).is_absolute()
}

/// 返回路径分隔符（Windows: `\`, Unix: `/`）
pub fn sep() -> String {
    std::path::MAIN_SEPARATOR.to_string()
}

/// 判断路径是否为文件
pub fn is_file(path: String) -> bool {
    let path_buf = Path::new(&path);
    // 先检查路径是否存在
    if !path_buf.exists() {
        eprintln!("[DEBUG] is_file: Path does not exist: {}", path);
        return false;
    }
    let result = path_buf.is_file();
    eprintln!("[DEBUG] is_file: path={}, exists={}, is_file={}", path, path_buf.exists(), result);
    result
}

/// 判断路径是否为目录
pub fn is_dir(path: String) -> bool {
    let path_buf = Path::new(&path);
    // 先检查路径是否存在
    if !path_buf.exists() {
        eprintln!("[DEBUG] is_dir: Path does not exist: {}", path);
        return false;
    }
    let result = path_buf.is_dir();
    eprintln!("[DEBUG] is_dir: path={}, exists={}, is_dir={}", path, path_buf.exists(), result);
    result
}

/// 判断路径是否存在
pub fn exists(path: String) -> bool {
    Path::new(&path).exists()
}
