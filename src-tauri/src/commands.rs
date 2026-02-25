use crate::js_engine::JsEngine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameResult {
    pub original: String,
    pub new_name: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewResult {
    pub mappings: Vec<(String, String)>,
    pub errors: Vec<String>,
}

/// 预览重命名结果（不实际重命名）
#[tauri::command]
pub async fn preview_rename(
    files: Vec<String>,
    script: String,
    engine: State<'_, JsEngine>,
) -> Result<PreviewResult, String> {
    if files.is_empty() {
        return Ok(PreviewResult {
            mappings: vec![],
            errors: vec!["文件列表为空".to_string()],
        });
    }

    if script.trim().is_empty() {
        return Ok(PreviewResult {
            mappings: vec![],
            errors: vec!["脚本不能为空".to_string()],
        });
    }

    // 提取文件名（如果传入的是完整路径）
    let file_names: Vec<String> = files
        .iter()
        .map(|f| {
            Path::new(f)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| f.clone())
        })
        .collect();

    match engine.execute_rename_batch(&file_names, &script) {
        Ok(mappings) => {
            let mut errors = Vec::new();
            // 映射回原始路径
            // mappings 是 (文件名, 新文件名) 的列表
            // files 是完整路径列表
            let valid_mappings: Vec<(String, String)> = mappings
                .into_iter()
                .zip(files.iter())
                .filter_map(|((_old_name, new_name), original_path)| {
                    if new_name.starts_with("ERROR:") {
                        errors.push(format!("{}: {}", original_path, new_name));
                        None
                    } else {
                        // original_path 是完整路径，new_name 是 JS 引擎返回的新文件名
                        Some((original_path.clone(), new_name))
                    }
                })
                .collect();

            Ok(PreviewResult {
                mappings: valid_mappings,
                errors,
            })
        }
        Err(e) => Err(format!("执行脚本失败: {}", e)),
    }
}

/// 执行重命名操作
#[tauri::command]
pub async fn execute_rename(
    files: Vec<String>,
    script: String,
    base_path: Option<String>,
    engine: State<'_, JsEngine>,
) -> Result<Vec<RenameResult>, String> {
    if files.is_empty() {
        return Err("文件列表为空".to_string());
    }

    if script.trim().is_empty() {
        return Err("脚本不能为空".to_string());
    }

    // 提取文件名（如果传入的是完整路径）
    let file_names: Vec<String> = files
        .iter()
        .map(|f| {
            Path::new(f)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| f.clone())
        })
        .collect();

    // 直接使用引擎获取预览结果
    let mappings = engine.execute_rename_batch(&file_names, &script)
        .map_err(|e| format!("执行脚本失败: {}", e))?;

    // 映射回原始路径
    let mappings: Vec<(String, String)> = mappings
        .into_iter()
        .zip(files.iter())
        .map(|((_old_name, new_name), original_path)| {
            (original_path.clone(), new_name)
        })
        .collect();

    let mut results = Vec::new();

    for (original, new_name) in mappings {
        // 跳过错误结果
        if new_name.starts_with("ERROR:") {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some(new_name.clone()),
            });
            continue;
        }
        // 验证新文件名
        if !is_valid_filename(&new_name) {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some("文件名包含非法字符".to_string()),
            });
            continue;
        }

        // 构建完整路径（防止路径遍历攻击）
        let old_path = if let Some(base) = &base_path {
            let base = PathBuf::from(base);
            let joined = base.join(&original);
            // 验证路径在基础目录内
            if !joined.starts_with(&base) {
                results.push(RenameResult {
                    original: original.clone(),
                    new_name: new_name.clone(),
                    success: false,
                    error: Some("路径遍历攻击检测".to_string()),
                });
                continue;
            }
            joined
        } else {
            // 当 base_path 为 None 时，original 应该是完整路径
            PathBuf::from(&original)
        };

        let new_path = if let Some(base) = &base_path {
            let base = PathBuf::from(base);
            let joined = base.join(&new_name);
            // 验证路径在基础目录内
            if !joined.starts_with(&base) {
                results.push(RenameResult {
                    original: original.clone(),
                    new_name: new_name.clone(),
                    success: false,
                    error: Some("路径遍历攻击检测".to_string()),
                });
                continue;
            }
            joined
        } else {
            // 当 base_path 为 None 时，从原始路径提取目录，然后与新文件名组合
            let old_path_buf = PathBuf::from(&original);
            if let Some(parent) = old_path_buf.parent() {
                parent.join(&new_name)
            } else {
                // 如果没有父目录，直接使用新文件名
                PathBuf::from(&new_name)
            }
        };

        // 检查文件是否存在
        if !old_path.exists() {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some("源文件不存在".to_string()),
            });
            continue;
        }

        // 检查目标文件是否已存在
        if new_path.exists() && old_path != new_path {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some("目标文件已存在".to_string()),
            });
            continue;
        }

        // 执行重命名
        match std::fs::rename(&old_path, &new_path) {
            Ok(_) => {
                results.push(RenameResult {
                    original: original.clone(),
                    new_name: new_name.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                results.push(RenameResult {
                    original: original.clone(),
                    new_name: new_name.clone(),
                    success: false,
                    error: Some(format!("重命名失败: {}", e)),
                });
            }
        }
    }

    Ok(results)
}

/// 获取文件夹内所有文件
#[tauri::command]
pub async fn get_folder_files(folder_path: String) -> Result<Vec<String>, String> {
    let path = Path::new(&folder_path);

    // 验证路径存在
    if !path.exists() {
        return Err("文件夹不存在".to_string());
    }

    // 验证是目录
    if !path.is_dir() {
        return Err("路径不是文件夹".to_string());
    }

    // 验证路径是绝对路径（安全性检查）
    if !path.is_absolute() {
        return Err("路径必须是绝对路径".to_string());
    }

    let mut files = Vec::new();

    match std::fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(file_name) = path.file_name() {
                                files.push(file_name.to_string_lossy().to_string());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("读取目录项失败: {}", e);
                    }
                }
            }
        }
        Err(e) => return Err(format!("读取文件夹失败: {}", e)),
    }

    files.sort();
    Ok(files)
}

/// 验证文件名是否合法
fn is_valid_filename(filename: &str) -> bool {
    if filename.is_empty() || filename.len() > 255 {
        return false;
    }

    // Windows 和 Unix 的非法字符
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if filename.chars().any(|c| invalid_chars.contains(&c)) {
        return false;
    }

    // Windows 保留名称
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    
    let upper_name = filename.to_uppercase();
    if reserved_names.iter().any(|&name| upper_name == name || upper_name.starts_with(&format!("{} ", name))) {
        return false;
    }

    // 不能以点或空格开头/结尾（Windows）
    if filename.trim() != filename {
        return false;
    }

    true
}
