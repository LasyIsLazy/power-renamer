use anyhow::{Context, Result};
use std::fs::read_dir;
use std::path::Path;

/// 读取文件夹内的所有文件（仅文件，不包括子文件夹）
/// 返回文件的完整路径列表
pub fn read_dir_files(dir_path: String) -> Result<Vec<String>> {
    let path = Path::new(&dir_path);
    
    if !path.exists() {
        anyhow::bail!("Directory does not exist: {}", dir_path);
    }
    
    if !path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", dir_path);
    }
    
    let entries = read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", dir_path))?;
    
    let mut files = Vec::new();
    
    for entry in entries {
        let entry = entry
            .with_context(|| format!("Failed to read directory entry in: {}", dir_path))?;
        let file_path = entry.path();
        
        if file_path.is_file() {
            files.push(file_path.to_string_lossy().to_string());
        }
    }
    
    files.sort();
    Ok(files)
}

/// 递归读取文件夹内的所有文件（包括子文件夹中的文件）
/// 返回所有文件的完整路径列表
pub fn read_dir_files_recursive(dir_path: String) -> Result<Vec<String>> {
    let path = Path::new(&dir_path);
    
    if !path.exists() {
        anyhow::bail!("Directory does not exist: {}", dir_path);
    }
    
    if !path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", dir_path);
    }
    
    let mut files = Vec::new();
    read_dir_recursive(path, &mut files)?;
    
    files.sort();
    Ok(files)
}

/// 递归读取目录的辅助函数
fn read_dir_recursive(dir_path: &Path, files: &mut Vec<String>) -> Result<()> {
    let entries = read_dir(dir_path)
        .with_context(|| format!("Failed to read directory: {}", dir_path.display()))?;
    
    for entry in entries {
        let entry = entry
            .with_context(|| format!("Failed to read directory entry in: {}", dir_path.display()))?;
        let file_path = entry.path();
        
        if file_path.is_file() {
            files.push(file_path.to_string_lossy().to_string());
        } else if file_path.is_dir() {
            // 递归处理子文件夹
            read_dir_recursive(&file_path, files)?;
        }
    }
    
    Ok(())
}
