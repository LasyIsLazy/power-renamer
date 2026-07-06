use crate::js_engine::{ExecuteOptions, JsEngine};
use crate::js_utils::Md5PreviewMode;
use crate::preview_session::{OperationKind, PreviewCompletePayload, PreviewSession};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenameResult {
    pub original: String,
    pub new_name: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCompletePayload {
    pub results: Vec<RenameResult>,
    pub cancelled: bool,
    pub error: Option<String>,
}

fn get_logs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let logs_dir = app_data_dir.join("logs");
    fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("Failed to create logs directory: {}", e))?;

    Ok(logs_dir)
}

fn today_log_file(logs_dir: &Path) -> PathBuf {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    logs_dir.join(format!("{}.log", date))
}

fn format_log_entry(context: &str, message: &str) -> String {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let sanitized = message.replace(['\n', '\r'], " ");
    format!("[{}] [{}] {}", timestamp, context, sanitized)
}

fn persist_script_logs(
    app: &tauri::AppHandle,
    context: &str,
    logs: &[String],
) -> Result<(), String> {
    if logs.is_empty() {
        return Ok(());
    }

    let logs_dir = get_logs_dir(app)?;
    let log_file = today_log_file(&logs_dir);
    let mut content = String::new();
    for log in logs {
        content.push_str(&format_log_entry(context, log));
        content.push('\n');
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write log file: {}", e))?;

    Ok(())
}

/// 加载脚本日志（默认加载当天，可按 YYYY-MM-DD 指定日期）
#[tauri::command]
pub async fn load_script_logs(
    app: tauri::AppHandle,
    date: Option<String>,
) -> Result<Vec<String>, String> {
    let logs_dir = get_logs_dir(&app)?;
    let log_file = if let Some(d) = date {
        logs_dir.join(format!("{}.log", d))
    } else {
        today_log_file(&logs_dir)
    };

    if !log_file.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&log_file)
        .map_err(|e| format!("Failed to read log file: {}", e))?;

    Ok(content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect())
}

/// 获取日志目录路径（用于显示给用户）
#[tauri::command]
pub async fn get_logs_dir_display(app: tauri::AppHandle) -> Result<String, String> {
    let logs_dir = get_logs_dir(&app)?;
    Ok(logs_dir.to_string_lossy().to_string())
}

fn count_preview_files(paths: &[String]) -> Result<usize, String> {
    let mut count = 0usize;
    for p in paths {
        let path = Path::new(p);
        if path.is_dir() {
            count += count_files_recursive(path)?;
        } else if path.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

/// 预览模式下是否会跳过 MD5 计算
fn will_skip_md5_in_preview(total_file_count: usize, mode: Md5PreviewMode) -> bool {
    match mode {
        Md5PreviewMode::Always => false,
        Md5PreviewMode::Skip => true,
        Md5PreviewMode::Auto => {
            total_file_count > crate::js_utils::md5::PREVIEW_FILE_COUNT_THRESHOLD
        }
    }
}

fn build_preview_new_path(original_path: &str, new_name: &str) -> String {
    if PathBuf::from(new_name).is_absolute() {
        new_name.to_string()
    } else {
        let original_path_buf = PathBuf::from(original_path);
        if let Some(parent) = original_path_buf.parent() {
            parent.join(new_name).to_string_lossy().to_string()
        } else {
            new_name.to_string()
        }
    }
}

/// 将脚本返回的映射转为预览结果，返回 (有效映射, 错误列表, 是否出错)
fn normalize_preview_mappings(
    mappings: Vec<(String, String)>,
) -> (Vec<(String, String)>, Vec<String>, bool) {
    let mut result_mappings = Vec::new();
    let mut errors = Vec::new();
    let mut has_error = false;

    for (original_path, new_name) in mappings {
        if new_name.starts_with("ERROR:") {
            errors.push(format!("{}: {}", original_path, new_name));
            has_error = true;
            break;
        }

        let new_path = build_preview_new_path(&original_path, &new_name);
        let original_path_buf = PathBuf::from(&original_path);
        let new_path_buf = PathBuf::from(&new_path);

        if original_path_buf != new_path_buf {
            result_mappings.push((original_path, new_path));
        }
    }

    (result_mappings, errors, has_error)
}

/// 取消正在进行的预览
#[tauri::command]
pub async fn cancel_preview(session: State<'_, PreviewSession>) -> Result<(), String> {
    session.cancel();
    Ok(())
}

/// 在后台线程执行预览，避免阻塞 IPC 导致进度事件无法及时送达前端
fn run_preview_work(
    app: tauri::AppHandle,
    engine: JsEngine,
    session: PreviewSession,
    full_paths: Vec<String>,
    script: String,
    params: Option<HashMap<String, serde_json::Value>>,
    total_file_count: usize,
    md5_mode: Md5PreviewMode,
) {
    let mut md5_preview_placeholder = will_skip_md5_in_preview(total_file_count, md5_mode);
    let execute_options = ExecuteOptions {
        preview_mode: true,
        total_file_count,
        preview_md5_mode: md5_mode,
        session: Some(session.clone()),
    };

    session.emit_progress(0, total_file_count, None, "start");

    let mut all_mappings = Vec::new();
    let mut errors = Vec::new();
    let mut all_logs = Vec::new();
    let mut cancelled = false;

    for file_path in &full_paths {
        if session.is_cancelled() {
            cancelled = true;
            eprintln!("[preview] cancelled before processing {}", file_path);
            break;
        }

        session.emit_progress(
            session.files_done(),
            total_file_count,
            Some(file_path.clone()),
            "script",
        );

        eprintln!("[preview] executing script for {}", file_path);

        match engine.execute_rename_single(file_path, &script, params.as_ref(), execute_options.clone())
        {
            Ok(mappings) => {
                let logs = engine.get_logs();
                let (chunk_mappings, chunk_errors, has_error) = normalize_preview_mappings(mappings);

                if !logs.is_empty() {
                    all_logs.extend(logs.clone());
                }

                if !chunk_mappings.is_empty() {
                    all_mappings.extend(chunk_mappings.clone());
                    session.emit_chunk(chunk_mappings, logs);
                }

                if has_error {
                    errors.extend(chunk_errors);
                    break;
                }
            }
            Err(e) => {
                eprintln!("[preview] script error for {}: {}", file_path, e);
                if session.is_cancelled() {
                    cancelled = true;
                } else {
                    errors.push(format!("{}: {}", file_path, e));
                }
                break;
            }
        }
    }

    if session.is_cancelled() {
        cancelled = true;
    }

    if let Err(e) = persist_script_logs(&app, "preview", &all_logs) {
        eprintln!("[preview] failed to persist script logs: {}", e);
    }

    if !md5_preview_placeholder {
        md5_preview_placeholder = all_mappings.iter().any(|(_, new_name)| {
            new_name.contains(crate::js_utils::md5::PREVIEW_PLACEHOLDER)
        });
    }

    session.emit_preview_complete(PreviewCompletePayload {
        md5_preview_placeholder,
        errors,
        cancelled,
    });
    session.finish();
}

/// 预览重命名结果（通过事件增量推送，不实际重命名）
#[tauri::command]
pub async fn preview_rename(
    app: tauri::AppHandle,
    files: Vec<String>,
    script: String,
    base_path: Option<String>,
    params: Option<HashMap<String, serde_json::Value>>,
    preview_md5_mode: Option<String>,
    engine: State<'_, JsEngine>,
    session: State<'_, PreviewSession>,
) -> Result<(), String> {
    if files.is_empty() {
        PreviewSession::emit_preview_complete_via_app(
            &app,
            PreviewCompletePayload {
                md5_preview_placeholder: false,
                errors: vec!["文件列表为空".to_string()],
                cancelled: false,
            },
        );
        return Ok(());
    }

    if script.trim().is_empty() {
        PreviewSession::emit_preview_complete_via_app(
            &app,
            PreviewCompletePayload {
                md5_preview_placeholder: false,
                errors: vec!["脚本不能为空".to_string()],
                cancelled: false,
            },
        );
        return Ok(());
    }

    // 构建完整路径列表（如果有 base_path，需要拼接）
    let full_paths: Vec<String> = if let Some(base) = &base_path {
        files
            .iter()
            .map(|f| {
                // 如果 f 已经是绝对路径，直接使用；否则与 base_path 拼接
                let f_path = PathBuf::from(f);
                if f_path.is_absolute() {
                    f.to_string()
                } else {
                    let base_path = PathBuf::from(base);
                    base_path.join(f).to_string_lossy().to_string()
                }
            })
            .collect()
    } else {
        files.clone()
    };

    // 执行前校验：所有路径必须存在，否则立即返回错误并终止
    for p in &full_paths {
        let path_buf = PathBuf::from(p);
        if !path_buf.exists() {
            PreviewSession::emit_preview_complete_via_app(
                &app,
                PreviewCompletePayload {
                    md5_preview_placeholder: false,
                    errors: vec![format!("路径不存在: {}", p)],
                    cancelled: false,
                },
            );
            return Ok(());
        }
    }

    let total_file_count = count_preview_files(&full_paths)?;
    session.try_begin(app.clone(), total_file_count, OperationKind::Preview)?;

    let md5_mode = Md5PreviewMode::from_str(preview_md5_mode.as_deref().unwrap_or("auto"));
    let preview_session = (*session).clone();
    let engine = (*engine).clone();
    let script_clone = script.clone();
    let params_clone = params.clone();
    let full_paths_clone = full_paths.clone();
    let app_clone = app.clone();

    eprintln!(
        "[preview] spawning worker thread, paths={}, total_files={}",
        full_paths_clone.len(),
        total_file_count
    );

    std::thread::spawn(move || {
        run_preview_work(
            app_clone,
            engine,
            preview_session,
            full_paths_clone,
            script_clone,
            params_clone,
            total_file_count,
            md5_mode,
        );
    });

    Ok(())
}

/// 执行重命名核心逻辑
fn run_execute_core(
    app: &tauri::AppHandle,
    engine: &JsEngine,
    session: Option<&PreviewSession>,
    full_paths: &[String],
    script: &str,
    params: Option<&HashMap<String, serde_json::Value>>,
    base_path: &Option<String>,
) -> Result<Vec<RenameResult>, String> {
    let total_file_count = count_preview_files(full_paths)?;
    if let Some(s) = session {
        s.emit_progress(0, total_file_count, None, "start");
    }

    let execute_options = ExecuteOptions {
        preview_mode: false,
        total_file_count,
        preview_md5_mode: Md5PreviewMode::Always,
        session: session.cloned(),
    };

    let mut all_mappings = Vec::new();
    let mut all_logs = Vec::new();

    for file_path in full_paths {
        if session.is_some_and(|s| s.is_cancelled()) {
            break;
        }

        if let Some(s) = session {
            s.emit_progress(
                s.files_done(),
                total_file_count,
                Some(file_path.clone()),
                "script",
            );
        }

        match engine.execute_rename_single(file_path, script, params, execute_options.clone()) {
            Ok(mappings) => {
                let logs = engine.get_logs();
                all_logs.extend(logs);
                let has_error = mappings.iter().any(|(_, new_name)| new_name.starts_with("ERROR:"));
                all_mappings.extend(mappings);
                if has_error {
                    break;
                }
            }
            Err(e) => {
                all_mappings.push((file_path.clone(), format!("ERROR: {}", e)));
                break;
            }
        }
    }

    if let Err(e) = persist_script_logs(app, "execute", &all_logs) {
        eprintln!("[execute] failed to persist script logs: {}", e);
    }

    let rename_total = all_mappings.len();
    if let Some(s) = session {
        s.emit_progress(0, rename_total.max(1), None, "rename");
    }

    let mut results = Vec::new();
    let mut renamed_folders: std::collections::HashMap<PathBuf, PathBuf> =
        std::collections::HashMap::new();

    for (rename_index, (original, new_name)) in all_mappings.into_iter().enumerate() {
        if session.is_some_and(|s| s.is_cancelled()) {
            break;
        }

        if let Some(s) = session {
            s.emit_progress(
                rename_index,
                rename_total.max(1),
                Some(original.clone()),
                "rename",
            );
        }

        if new_name.starts_with("ERROR:") {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some(new_name.clone()),
            });
            continue;
        }

        let original_path_buf = PathBuf::from(&original);
        let old_path = if original_path_buf.is_absolute() {
            if let Some(base) = base_path {
                let base = PathBuf::from(base);
                if !original_path_buf.starts_with(&base) {
                    results.push(RenameResult {
                        original: original.clone(),
                        new_name: new_name.clone(),
                        success: false,
                        error: Some("路径遍历攻击检测".to_string()),
                    });
                    continue;
                }
            }
            original_path_buf
        } else if let Some(base) = base_path {
            let base = PathBuf::from(base);
            let joined = base.join(&original);
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
            original_path_buf
        };

        let new_path_buf = PathBuf::from(&new_name);
        let new_path = if new_path_buf.is_absolute() {
            if let Some(base) = base_path {
                let base = PathBuf::from(base);
                if !new_path_buf.starts_with(&base) {
                    results.push(RenameResult {
                        original: original.clone(),
                        new_name: new_name.clone(),
                        success: false,
                        error: Some("路径遍历攻击检测".to_string()),
                    });
                    continue;
                }
            }
            new_path_buf
        } else if let Some(base) = base_path {
            let base = PathBuf::from(base);
            let joined = base.join(&new_name);
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
        } else if let Some(parent) = old_path.parent() {
            parent.join(&new_name)
        } else {
            PathBuf::from(&new_name)
        };

        if let Some(file_name) = new_path.file_name() {
            if let Some(file_name_str) = file_name.to_str() {
                if !is_valid_filename(file_name_str) {
                    results.push(RenameResult {
                        original: original.clone(),
                        new_name: new_name.clone(),
                        success: false,
                        error: Some("文件名包含非法字符".to_string()),
                    });
                    continue;
                }
            } else {
                results.push(RenameResult {
                    original: original.clone(),
                    new_name: new_name.clone(),
                    success: false,
                    error: Some("文件名包含非法字符（无法转换为字符串）".to_string()),
                });
                continue;
            }
        } else {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some("无效的文件路径".to_string()),
            });
            continue;
        }

        if old_path == new_path {
            continue;
        }

        let mut actual_old_path = old_path.clone();
        if !actual_old_path.exists() {
            if let Some(parent) = actual_old_path.parent() {
                if let Some(new_parent) = renamed_folders.get(parent) {
                    if let Some(file_name) = actual_old_path.file_name() {
                        actual_old_path = new_parent.join(file_name);
                    }
                } else {
                    let mut current_parent = parent;
                    let mut updated_path = actual_old_path.clone();
                    while let Some(grandparent) = current_parent.parent() {
                        if let Some(new_grandparent) = renamed_folders.get(grandparent) {
                            let relative_path = actual_old_path
                                .strip_prefix(grandparent)
                                .unwrap_or(&actual_old_path);
                            updated_path = new_grandparent.join(relative_path);
                            break;
                        }
                        current_parent = grandparent;
                    }
                    actual_old_path = updated_path;
                }
            }
        }

        if !actual_old_path.exists() {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some("源文件不存在".to_string()),
            });
            continue;
        }

        if new_path.exists() && actual_old_path != new_path {
            results.push(RenameResult {
                original: original.clone(),
                new_name: new_name.clone(),
                success: false,
                error: Some("目标文件已存在".to_string()),
            });
            continue;
        }

        let is_folder = actual_old_path.is_dir();

        match std::fs::rename(&actual_old_path, &new_path) {
            Ok(_) => {
                if is_folder {
                    renamed_folders.insert(old_path.clone(), new_path.clone());
                }
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

        if let Some(s) = session {
            s.emit_progress(
                rename_index + 1,
                rename_total.max(1),
                Some(original),
                "rename",
            );
        }
    }

    Ok(results)
}

fn run_execute_work(
    app: tauri::AppHandle,
    engine: JsEngine,
    session: PreviewSession,
    full_paths: Vec<String>,
    script: String,
    params: Option<HashMap<String, serde_json::Value>>,
    base_path: Option<String>,
) {
    let cancelled = session.is_cancelled();
    let outcome = run_execute_core(
        &app,
        &engine,
        Some(&session),
        &full_paths,
        &script,
        params.as_ref(),
        &base_path,
    );

    let payload = match outcome {
        Ok(results) => ExecuteCompletePayload {
            results,
            cancelled: session.is_cancelled() || cancelled,
            error: None,
        },
        Err(e) => ExecuteCompletePayload {
            results: vec![],
            cancelled: session.is_cancelled(),
            error: Some(e),
        },
    };

    session.emit_execute_complete(payload);
    session.finish();
}

/// 执行重命名操作
#[tauri::command]
pub async fn execute_rename(
    app: tauri::AppHandle,
    files: Vec<String>,
    script: String,
    base_path: Option<String>,
    params: Option<HashMap<String, serde_json::Value>>,
    silent: Option<bool>,
    engine: State<'_, JsEngine>,
    session: State<'_, PreviewSession>,
) -> Result<Option<Vec<RenameResult>>, String> {
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
                // 如果 f 已经是绝对路径，直接使用；否则与 base_path 拼接
                let f_path = PathBuf::from(f);
                if f_path.is_absolute() {
                    f.to_string()
                } else {
                    let base_path = PathBuf::from(base);
                    base_path.join(f).to_string_lossy().to_string()
                }
            })
            .collect()
    } else {
        files.clone()
    };

    // 执行前校验：所有路径必须存在，否则立即返回错误并终止
    for p in &full_paths {
        let path_buf = PathBuf::from(p);
        if !path_buf.exists() {
            return Err(format!("路径不存在: {}", p));
        }
    }

    if silent.unwrap_or(false) {
        let engine = (*engine).clone();
        let results = run_execute_core(
            &app,
            &engine,
            None,
            &full_paths,
            &script,
            params.as_ref(),
            &base_path,
        )?;
        return Ok(Some(results));
    }

    let total_file_count = count_preview_files(&full_paths)?;
    session.try_begin(app.clone(), total_file_count, OperationKind::Execute)?;

    let engine = (*engine).clone();
    let preview_session = (*session).clone();
    let script_clone = script.clone();
    let params_clone = params.clone();
    let full_paths_clone = full_paths.clone();
    let base_path_clone = base_path.clone();
    let app_clone = app.clone();

    eprintln!(
        "[execute] spawning worker thread, paths={}, total_files={}",
        full_paths_clone.len(),
        total_file_count
    );

    std::thread::spawn(move || {
        run_execute_work(
            app_clone,
            engine,
            preview_session,
            full_paths_clone,
            script_clone,
            params_clone,
            base_path_clone,
        );
    });

    Ok(None)
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

fn count_files_recursive(path: &Path) -> Result<usize, String> {
    if !path.exists() {
        return Err("文件夹不存在".to_string());
    }
    if !path.is_dir() {
        return Err("路径不是文件夹".to_string());
    }
    if !path.is_absolute() {
        return Err("路径必须是绝对路径".to_string());
    }

    let mut count = 0usize;
    let entries = std::fs::read_dir(path).map_err(|e| format!("读取文件夹失败: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let entry_path = entry.path();
        if entry_path.is_file() {
            count += 1;
        } else if entry_path.is_dir() {
            count += count_files_recursive(&entry_path)?;
        }
    }
    Ok(count)
}

/// 递归统计文件夹内文件数量
#[tauri::command]
pub async fn count_folder_files_recursive(folder_path: String) -> Result<usize, String> {
    count_files_recursive(Path::new(&folder_path))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub files: Vec<serde_json::Value>,
    pub script: String,
    pub results: Vec<RenameResult>,
}

fn get_history_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    Ok(app_data_dir.join("history.json"))
}

/// 保存历史记录（最多保留 50 条）
#[tauri::command]
pub async fn save_history(
    app: tauri::AppHandle,
    history: Vec<HistoryEntry>,
) -> Result<(), String> {
    let history_file = get_history_file(&app)?;
    let trimmed: Vec<HistoryEntry> = history.into_iter().take(50).collect();
    let json = serde_json::to_string_pretty(&trimmed)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;
    fs::write(&history_file, json)
        .map_err(|e| format!("Failed to write history file: {}", e))?;
    Ok(())
}

/// 加载历史记录
#[tauri::command]
pub async fn load_history(app: tauri::AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let history_file = get_history_file(&app)?;
    if !history_file.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&history_file)
        .map_err(|e| format!("Failed to read history file: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse history: {}", e))
}

/// 获取文件夹内所有子文件夹
#[tauri::command]
pub async fn get_folder_dirs(folder_path: String) -> Result<Vec<String>, String> {
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

    let mut dirs = Vec::new();

    match std::fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(dir_name) = path.file_name() {
                                dirs.push(dir_name.to_string_lossy().to_string());
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

    dirs.sort();
    Ok(dirs)
}

/// 脚本模板结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptTemplate {
    pub id: String,
    pub name: String,
    pub script: String,
    pub created_at: String,
    pub updated_at: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Option<Vec<ScriptParameter>>,
}

/// 脚本参数结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptParameter {
    pub name: String,                    // 参数名
    #[serde(rename = "type")]
    pub param_type: String,              // "string" | "number" | "boolean"
    pub default: Option<serde_json::Value>, // 默认值
    pub description: Option<String>,      // 参数描述
    pub required: bool,                  // 是否必填
}

/// 脚本 Manifest 结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptManifest {
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Option<Vec<ScriptParameter>>, // 脚本参数定义
}

/// 获取脚本目录路径
fn get_scripts_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let scripts_dir = app_data_dir.join("scripts");
    
    // 确保目录存在
    fs::create_dir_all(&scripts_dir)
        .map_err(|e| format!("Failed to create scripts directory: {}", e))?;
    
    Ok(scripts_dir)
}

/// 获取脚本文件路径
fn get_script_file_path(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let scripts_dir = get_scripts_dir(app)?;
    
    // 清理文件名，移除非法字符
    let safe_name = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>();
    
    Ok(scripts_dir.join(format!("{}.js", safe_name)))
}

/// 获取脚本 Manifest 文件路径
fn get_script_manifest_path(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let scripts_dir = get_scripts_dir(app)?;
    
    // 清理文件名，移除非法字符
    let safe_name = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>();
    
    Ok(scripts_dir.join(format!("{}.manifest.json", safe_name)))
}

/// 获取脚本目录路径（用于显示给用户）
#[tauri::command]
pub async fn get_scripts_dir_display(app: tauri::AppHandle) -> Result<String, String> {
    let scripts_dir = get_scripts_dir(&app)?;
    Ok(scripts_dir.to_string_lossy().to_string())
}

/// 加载保存的脚本模板（从文件系统读取所有 .js 文件）
#[tauri::command]
pub async fn load_saved_scripts(app: tauri::AppHandle) -> Result<Vec<ScriptTemplate>, String> {
    let scripts_dir = get_scripts_dir(&app)?;
    
    if !scripts_dir.exists() {
        return Ok(vec![]);
    }
    
    let mut scripts = Vec::new();
    
    match fs::read_dir(&scripts_dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                if ext == "js" {
                                    if let Some(file_name) = path.file_stem() {
                                        let name = file_name.to_string_lossy().to_string();
                                        
                                        // 读取文件内容
                                        match fs::read_to_string(&path) {
                                            Ok(content) => {
                                                // 获取文件元数据
                                                let metadata = entry.metadata()
                                                    .map_err(|e| format!("Failed to get file metadata: {}", e))?;
                                                
                                                let created_at = metadata.created()
                                                    .or_else(|_| metadata.modified())
                                                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                                                    .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
                                                
                                                let updated_at = metadata.modified()
                                                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                                                    .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
                                                
                                                // 读取 Manifest 文件
                                                let manifest_path = get_script_manifest_path(&app, &name)?;
                                                let (display_name, description, parameters) = if manifest_path.exists() {
                                                    match fs::read_to_string(&manifest_path) {
                                                        Ok(manifest_content) => {
                                                            match serde_json::from_str::<ScriptManifest>(&manifest_content) {
                                                                Ok(manifest) => (
                                                                    Some(manifest.display_name),
                                                                    Some(manifest.description),
                                                                    manifest.parameters,
                                                                ),
                                                                Err(e) => {
                                                                    eprintln!("Failed to parse manifest file {}: {}", manifest_path.display(), e);
                                                                    (None, None, None)
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            eprintln!("Failed to read manifest file {}: {}", manifest_path.display(), e);
                                                            (None, None, None)
                                                        }
                                                    }
                                                } else {
                                                    (None, None, None)
                                                };
                                                
                                                // 使用文件名作为 ID
                                                let id = name.clone();
                                                
                                                scripts.push(ScriptTemplate {
                                                    id,
                                                    name,
                                                    script: content,
                                                    created_at,
                                                    updated_at,
                                                    display_name,
                                                    description,
                                                    parameters,
                                                });
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to read script file {}: {}", path.display(), e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read directory entry: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Failed to read scripts directory: {}", e));
        }
    }
    
    // 按更新时间排序（最新的在前）
    scripts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    
    Ok(scripts)
}

/// 保存脚本模板（保存为 .js 文件）
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
    
    let scripts_dir = get_scripts_dir(&app)?;
    let new_name = name.trim();
    
    // 检查名称是否已存在（排除当前编辑的脚本）
    if let Some(ref id) = script_id {
        // 如果是更新，检查新名称是否与其他脚本冲突
        let existing_path = scripts_dir.join(format!("{}.js", id));
        if existing_path.exists() && id != new_name {
            // 检查新名称是否已存在
            let new_path = get_script_file_path(&app, new_name)?;
            if new_path.exists() {
                return Err("脚本名称已存在".to_string());
            }
        }
    } else {
        // 新建脚本，检查名称是否已存在
        let new_path = get_script_file_path(&app, new_name)?;
        if new_path.exists() {
            return Err("脚本名称已存在".to_string());
        }
    }
    
    let now = chrono::Utc::now();
    let now_str = now.to_rfc3339();
    
    // 如果是更新现有脚本，需要处理重命名
    if let Some(ref id) = script_id {
        let old_path = scripts_dir.join(format!("{}.js", id));
        let new_path = get_script_file_path(&app, new_name)?;
        
        if old_path.exists() {
            // 如果名称改变，需要重命名文件
            if old_path != new_path {
                fs::rename(&old_path, &new_path)
                    .map_err(|e| format!("Failed to rename script file: {}", e))?;
                
                // 同时重命名 manifest 文件（如果存在）
                let old_manifest_path = get_script_manifest_path(&app, id)?;
                let new_manifest_path = get_script_manifest_path(&app, new_name)?;
                if old_manifest_path.exists() && old_manifest_path != new_manifest_path {
                    fs::rename(&old_manifest_path, &new_manifest_path)
                        .map_err(|e| format!("Failed to rename manifest file: {}", e))?;
                }
            }
            
            // 更新文件内容和修改时间
            fs::write(&new_path, script.trim())
                .map_err(|e| format!("Failed to write script file: {}", e))?;
            
            // 获取创建时间（从旧文件）
            let created_at = old_path.metadata()
                .and_then(|m| m.created())
                .or_else(|_| old_path.metadata().and_then(|m| m.modified()))
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                .unwrap_or_else(|_| now_str.clone());
            
            return Ok(ScriptTemplate {
                id: new_name.to_string(),
                name: new_name.to_string(),
                script: script.trim().to_string(),
                created_at,
                updated_at: now_str,
                display_name: None,
                description: None,
                parameters: None,
            });
        }
    }
    
    // 创建新脚本文件
    let file_path = get_script_file_path(&app, new_name)?;
    fs::write(&file_path, script.trim())
        .map_err(|e| format!("Failed to write script file: {}", e))?;
    
    Ok(ScriptTemplate {
        id: new_name.to_string(),
        name: new_name.to_string(),
        script: script.trim().to_string(),
        created_at: now_str.clone(),
        updated_at: now_str,
        display_name: None,
        description: None,
        parameters: None,
    })
}

/// 删除脚本模板（删除文件）
#[tauri::command]
pub async fn delete_script(
    app: tauri::AppHandle,
    script_id: String,
) -> Result<(), String> {
    let scripts_dir = get_scripts_dir(&app)?;
    let file_path = scripts_dir.join(format!("{}.js", script_id));
    
    if file_path.exists() {
        fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete script file: {}", e))?;
    }
    
    Ok(())
}

/// 重命名脚本模板（重命名文件）
#[tauri::command]
pub async fn rename_script(
    app: tauri::AppHandle,
    script_id: String,
    new_name: String,
) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("脚本名称不能为空".to_string());
    }
    
    let scripts_dir = get_scripts_dir(&app)?;
    let old_path = scripts_dir.join(format!("{}.js", script_id));
    let new_path = get_script_file_path(&app, &new_name.trim())?;
    
    if !old_path.exists() {
        return Err("脚本不存在".to_string());
    }
    
    if new_path.exists() && old_path != new_path {
        return Err("脚本名称已存在".to_string());
    }
    
    if old_path != new_path {
        fs::rename(&old_path, &new_path)
            .map_err(|e| format!("Failed to rename script file: {}", e))?;
        
        // 同时重命名 manifest 文件（如果存在）
        let old_manifest_path = get_script_manifest_path(&app, &script_id)?;
        let new_manifest_path = get_script_manifest_path(&app, &new_name.trim())?;
        if old_manifest_path.exists() && old_manifest_path != new_manifest_path {
            fs::rename(&old_manifest_path, &new_manifest_path)
                .map_err(|e| format!("Failed to rename manifest file: {}", e))?;
        }
    }
    
    Ok(())
}

/// 打开脚本文件（使用系统默认编辑器）
#[tauri::command]
pub async fn open_script_file(
    app: tauri::AppHandle,
    script_id: String,
) -> Result<(), String> {
    let scripts_dir = get_scripts_dir(&app)?;
    
    // 如果 script_id 是 "."，打开目录；否则打开文件
    let path = if script_id == "." {
        scripts_dir
    } else {
        let file_path = scripts_dir.join(format!("{}.js", script_id));
        if !file_path.exists() {
            return Err("脚本文件不存在".to_string());
        }
        file_path
    };
    
    // 使用系统默认程序打开文件或目录
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path.to_string_lossy().as_ref()])
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    
    Ok(())
}

/// 打开脚本 Manifest 文件（使用系统默认编辑器）
#[tauri::command]
pub async fn open_script_manifest(
    app: tauri::AppHandle,
    script_id: String,
) -> Result<(), String> {
    let manifest_path = get_script_manifest_path(&app, &script_id)?;
    
    // 如果 manifest 文件不存在，创建一个默认的
    if !manifest_path.exists() {
        let default_manifest = ScriptManifest {
            display_name: script_id.clone(),
            description: String::new(),
            parameters: None,
        };
        let manifest_json = serde_json::to_string_pretty(&default_manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        fs::write(&manifest_path, manifest_json)
            .map_err(|e| format!("Failed to create manifest file: {}", e))?;
    }
    
    // 使用系统默认程序打开文件
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", manifest_path.to_string_lossy().as_ref()])
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&manifest_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&manifest_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    
    Ok(())
}

/// 保存脚本 Manifest
#[tauri::command]
pub async fn save_script_manifest(
    app: tauri::AppHandle,
    script_id: String,
    display_name: String,
    description: String,
) -> Result<(), String> {
    if display_name.trim().is_empty() {
        return Err("中文名称不能为空".to_string());
    }
    
    let manifest_path = get_script_manifest_path(&app, &script_id)?;
    let manifest = ScriptManifest {
        display_name: display_name.trim().to_string(),
        description: description.trim().to_string(),
        parameters: None,
    };
    
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    
    fs::write(&manifest_path, manifest_json)
        .map_err(|e| format!("Failed to write manifest file: {}", e))?;
    
    Ok(())
}

/// 获取脚本 Manifest
#[tauri::command]
pub async fn get_script_manifest(
    app: tauri::AppHandle,
    script_id: String,
) -> Result<Option<ScriptManifest>, String> {
    let manifest_path = get_script_manifest_path(&app, &script_id)?;
    
    if !manifest_path.exists() {
        return Ok(None);
    }
    
    match fs::read_to_string(&manifest_path) {
        Ok(content) => {
            match serde_json::from_str::<ScriptManifest>(&content) {
                Ok(manifest) => Ok(Some(manifest)),
                Err(e) => Err(format!("Failed to parse manifest: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to read manifest: {}", e)),
    }
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
