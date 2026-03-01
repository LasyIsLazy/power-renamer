use crate::js_engine::JsEngine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use tauri::{State, Manager};

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

    // 对每个选择的文件/文件夹调用一次 rename
    let mut all_mappings = Vec::new();
    let mut errors = Vec::new();

    for file_path in &files {
        match engine.execute_rename_single(file_path, &script) {
            Ok(mappings) => {
                for (original_path, new_name) in mappings {
                    if new_name.starts_with("ERROR:") {
                        errors.push(format!("{}: {}", original_path, new_name));
                    } else {
                        all_mappings.push((original_path, new_name));
                    }
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", file_path, e));
            }
        }
    }

    Ok(PreviewResult {
        mappings: all_mappings,
        errors,
    })
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

    // 构建完整路径列表（如果有 base_path，需要拼接）
    let full_paths: Vec<String> = if let Some(base) = &base_path {
        files
            .iter()
            .map(|f| {
                let base_path = PathBuf::from(base);
                base_path.join(f).to_string_lossy().to_string()
            })
            .collect()
    } else {
        files.clone()
    };

    // 对每个选择的文件/文件夹调用一次 rename
    let mut all_mappings = Vec::new();

    for file_path in &full_paths {
        match engine.execute_rename_single(file_path, &script) {
            Ok(mappings) => {
                all_mappings.extend(mappings);
            }
            Err(e) => {
                // 如果失败，记录错误但继续处理其他文件
                eprintln!("Error processing {}: {}", file_path, e);
                all_mappings.push((file_path.clone(), format!("ERROR: {}", e)));
            }
        }
    }

    let mut results = Vec::new();

    for (original, new_name) in all_mappings {
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
            // 当 base_path 为 None 时，new_name 可能是完整路径或只是文件名
            // 检查 new_name 是否是绝对路径
            let new_path_buf = PathBuf::from(&new_name);
            if new_path_buf.is_absolute() {
                // 如果已经是完整路径，直接使用
                new_path_buf
            } else {
                // 如果只是文件名，从原始路径提取目录，然后与新文件名组合
                let old_path_buf = PathBuf::from(&original);
                if let Some(parent) = old_path_buf.parent() {
                    parent.join(&new_name)
                } else {
                    // 如果没有父目录，直接使用新文件名
                    PathBuf::from(&new_name)
                }
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

/// 脚本模板结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptTemplate {
    pub id: String,
    pub name: String,
    pub script: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 获取配置文件路径
fn get_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    // 确保目录存在
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    
    Ok(app_data_dir.join("scripts.json"))
}

/// 获取配置文件路径（用于显示给用户）
#[tauri::command]
pub async fn get_config_path_display(app: tauri::AppHandle) -> Result<String, String> {
    let config_path = get_config_path(&app)?;
    Ok(config_path.to_string_lossy().to_string())
}

/// 加载保存的脚本模板
#[tauri::command]
pub async fn load_saved_scripts(app: tauri::AppHandle) -> Result<Vec<ScriptTemplate>, String> {
    let config_path = get_config_path(&app)?;
    
    if !config_path.exists() {
        return Ok(vec![]);
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    let scripts: Vec<ScriptTemplate> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;
    
    Ok(scripts)
}

/// 保存脚本模板
#[tauri::command]
pub async fn save_script(
    app: tauri::AppHandle,
    name: String,
    script: String,
    script_id: Option<String>,
) -> Result<ScriptTemplate, String> {
    if name.trim().is_empty() {
        return Err("脚本名称不能为空".to_string());
    }
    
    if script.trim().is_empty() {
        return Err("脚本内容不能为空".to_string());
    }
    
    let config_path = get_config_path(&app)?;
    let mut scripts: Vec<ScriptTemplate> = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?
    } else {
        vec![]
    };
    
    let now = chrono::Utc::now().to_rfc3339();
    
    // 检查名称是否已存在（排除当前编辑的脚本）
    let check_id = script_id.as_ref().map(|s| s.as_str()).unwrap_or("");
    if scripts.iter().any(|s| s.name == name.trim() && s.id != check_id) {
        return Err("脚本名称已存在".to_string());
    }
    
    if let Some(ref id) = script_id {
        // 更新现有脚本
        if let Some(existing) = scripts.iter_mut().find(|s| s.id == *id) {
            existing.name = name.trim().to_string();
            existing.script = script.trim().to_string();
            existing.updated_at = now.clone();
            
            let updated = existing.clone();
            save_scripts_to_file(&config_path, &scripts)?;
            return Ok(updated);
        }
    }
    
    // 创建新脚本
    let new_script = ScriptTemplate {
        id: script_id.unwrap_or_else(|| chrono::Utc::now().timestamp_millis().to_string()),
        name: name.trim().to_string(),
        script: script.trim().to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    
    scripts.push(new_script.clone());
    save_scripts_to_file(&config_path, &scripts)?;
    
    Ok(new_script)
}

/// 删除脚本模板
#[tauri::command]
pub async fn delete_script(
    app: tauri::AppHandle,
    script_id: String,
) -> Result<(), String> {
    let config_path = get_config_path(&app)?;
    
    if !config_path.exists() {
        return Ok(());
    }
    
    let mut scripts: Vec<ScriptTemplate> = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))
        .and_then(|content| {
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config file: {}", e))
        })?;
    
    let initial_len = scripts.len();
    scripts.retain(|s| s.id != script_id);
    
    if scripts.len() < initial_len {
        save_scripts_to_file(&config_path, &scripts)?;
    }
    
    Ok(())
}

/// 重命名脚本模板
#[tauri::command]
pub async fn rename_script(
    app: tauri::AppHandle,
    script_id: String,
    new_name: String,
) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("脚本名称不能为空".to_string());
    }
    
    let config_path = get_config_path(&app)?;
    
    if !config_path.exists() {
        return Err("配置文件不存在".to_string());
    }
    
    let mut scripts: Vec<ScriptTemplate> = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))
        .and_then(|content| {
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config file: {}", e))
        })?;
    
    // 检查名称是否已存在
    if scripts.iter().any(|s| s.name == new_name.trim() && s.id != script_id) {
        return Err("脚本名称已存在".to_string());
    }
    
    if let Some(script) = scripts.iter_mut().find(|s| s.id == script_id) {
        script.name = new_name.trim().to_string();
        script.updated_at = chrono::Utc::now().to_rfc3339();
        save_scripts_to_file(&config_path, &scripts)?;
        Ok(())
    } else {
        Err("脚本不存在".to_string())
    }
}

/// 保存脚本列表到文件
fn save_scripts_to_file(path: &PathBuf, scripts: &[ScriptTemplate]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(scripts)
        .map_err(|e| format!("Failed to serialize scripts: {}", e))?;
    
    fs::write(path, content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    Ok(())
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
