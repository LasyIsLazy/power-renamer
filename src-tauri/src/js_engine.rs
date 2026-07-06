use anyhow::{Context, Result};
use rquickjs::{Context as JsContext, Function, Object, Runtime as JsRuntime, Value as JsValue};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::js_utils::{Md5Options, Md5PreviewMode};
use crate::preview_session::PreviewSession;

/// 脚本执行选项
#[derive(Clone, Default)]
pub struct ExecuteOptions {
    /// 是否为预览模式（预览模式下可能跳过耗时的 MD5 计算）
    pub preview_mode: bool,
    /// 待处理文件总数（用于判断是否跳过 MD5）
    pub total_file_count: usize,
    /// 预览时 MD5 计算策略
    pub preview_md5_mode: Md5PreviewMode,
    /// 预览会话（用于进度与取消）
    pub session: Option<PreviewSession>,
}

impl ExecuteOptions {
    pub fn md5_options(&self) -> Md5Options {
        Md5Options {
            preview_mode: self.preview_mode,
            total_file_count: self.total_file_count,
            preview_md5_mode: self.preview_md5_mode,
            session: self.session.clone(),
        }
    }
}

/// 重命名结果：可以是单个文件的新名称，或批量重命名映射
#[derive(Debug, Clone)]
pub enum RenameResult {
    /// 单个文件的新名称
    Single(String),
    /// 批量重命名映射：Vec<(原文件路径, 新文件名)>
    Batch(Vec<(String, String)>),
}

pub struct JsEngine {
    runtime: Arc<Mutex<JsRuntime>>,
    log_collector: crate::js_utils::LogCollector,
}

impl Clone for JsEngine {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            log_collector: self.log_collector.clone(),
        }
    }
}

unsafe impl Send for JsEngine {}
unsafe impl Sync for JsEngine {}

impl JsEngine {
    pub fn new() -> Result<Self> {
        let runtime = JsRuntime::new().context("Failed to create JS runtime")?;
        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            log_collector: crate::js_utils::LogCollector::new(),
        })
    }

    /// 执行重命名脚本
    /// file_path: 文件或文件夹的完整路径
    /// filename: 文件或文件夹名称
    /// params: 脚本参数
    /// 返回：单个文件的新名称，或批量重命名映射
    pub fn execute_rename(
        &self,
        filename: &str,
        script: &str,
        file_path: Option<&str>,
        params: Option<&HashMap<String, JsonValue>>,
        options: ExecuteOptions,
    ) -> Result<RenameResult> {
        if script.trim().is_empty() {
            anyhow::bail!("Script is empty");
        }

        let runtime = self.runtime.lock().unwrap();
        let context = JsContext::full(&runtime)
            .context("Failed to create JS context")?;

        // 清空之前的日志
        self.log_collector.clear();

        // 执行脚本
        let result = context.with(|ctx| {
            // 1. 首先注入 JS 工具函数到 __utils 对象（path、md5、fs、log）
            // 这些工具函数需要在编译脚本之前就注入，这样脚本编译时就能访问到
            crate::js_utils::setup_js_utils(&ctx, Some(&self.log_collector), options.md5_options())
                .with_context(|| {
                    format!(
                        "Failed to setup JS utils. This may be due to:\n\
                        1. QuickJS context initialization issues\n\
                        2. Console.log hijacking problems\n\
                        3. Function binding errors\n\
                        Please check the error details above."
                    )
                })?;
            
            // 2. 编译脚本（用户脚本直接定义 rename 函数）
            // 此时脚本已经可以访问 __utils.path、__utils.md5、__utils.fs 等工具函数
            ctx.eval::<(), _>(script)
                .context("Failed to compile JS script")?;
            
            // 3. 从全局作用域获取 rename 函数
            let rename_value: JsValue = ctx.globals()
                .get("rename")
                .context("Failed to get rename function from global scope. Make sure your script defines 'function rename() { ... }'")?;
            
            let rename_fn = Function::from_value(rename_value)
                .context("rename is not a function")?;
            
            // 4. 设置执行时的全局变量（每次调用可能不同）
            if let Some(path) = file_path {
                ctx.globals().set("__filePath", path)
                    .context("Failed to set __filePath")?;
            } else {
                ctx.eval::<(), _>("var __filePath = null;")
                    .context("Failed to set __filePath to null")?;
            }
            ctx.globals().set("__fileName", filename)
                .context("Failed to set __fileName")?;
            
            // 5. 注入参数到 JS 上下文（如果提供了参数）
            if let Some(params) = params {
                let params_obj = Object::new(ctx.clone())
                    .context("Failed to create params object")?;
                
                for (key, value) in params {
                    match value {
                        JsonValue::String(s) => {
                            params_obj.set(key.as_str(), s.as_str())
                                .with_context(|| format!("Failed to set param '{}' as string", key))?;
                        },
                        JsonValue::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                params_obj.set(key.as_str(), i as i32)
                                    .with_context(|| format!("Failed to set param '{}' as integer", key))?;
                            } else if let Some(f) = n.as_f64() {
                                params_obj.set(key.as_str(), f)
                                    .with_context(|| format!("Failed to set param '{}' as float", key))?;
                            }
                        },
                        JsonValue::Bool(b) => {
                            params_obj.set(key.as_str(), *b)
                                .with_context(|| format!("Failed to set param '{}' as boolean", key))?;
                        },
                        _ => {
                            // 其他类型转换为字符串
                            let s = serde_json::to_string(value)
                                .unwrap_or_else(|_| "".to_string());
                            params_obj.set(key.as_str(), s.as_str())
                                .with_context(|| format!("Failed to set param '{}' as string", key))?;
                        }
                    }
                }
                
                ctx.globals().set("__params", params_obj)
                    .context("Failed to set __params in global scope")?;
            } else {
                // 如果没有参数，设置一个空对象
                let empty_obj = Object::new(ctx.clone())
                    .context("Failed to create empty params object")?;
                ctx.globals().set("__params", empty_obj)
                    .context("Failed to set empty __params in global scope")?;
            }
            
            // 6. 调用 rename 函数
            let result: JsValue = rename_fn.call(())
                .context("Failed to call rename function")?;
            
            // 7. 将结果转换为 JSON 以便处理
            // 要求用户脚本返回对象格式 {原始路径: 新路径}
            let serialize_script = r#"
                (function() {
                    try {
                        var result = globalThis.__rename_result_temp;
                        
                        // 必须是对象格式
                        if (typeof result !== 'object' || result === null) {
                            throw new Error('rename function must return an object: {原始路径: 新路径}');
                        }
                        
                        return JSON.stringify(result);
                    } catch (error) {
                        var errorMsg = error.message || String(error);
                        if (error.stack) {
                            errorMsg += '\nStack: ' + error.stack;
                        }
                        return JSON.stringify({__error__: errorMsg});
                    }
                })()
            "#;
            ctx.globals().set("__rename_result_temp", result)?;
            let json_str: String = ctx.eval(serialize_script)
                .context("Failed to serialize JS result to JSON")?;
            
            // 8. 日志已经在 log_collector 中收集（通过 console.log 调用）
            // 在返回前检查日志数量（通过反射访问内部字段）
            // 注意：这里我们不能直接访问私有字段，所以先不检查
            
            // 9. 解析 JSON
            let result_value: JsonValue = serde_json::from_str(&json_str)
                .context("Failed to parse JS result as JSON")?;
            
            // 10. 处理结果
            self.parse_rename_result_from_json(result_value)
        })?;

        Ok(result)
    }
    
    /// 从 JSON 解析重命名函数的返回值
    /// 统一格式：对象 {原始路径: 新路径}
    fn parse_rename_result_from_json(&self, result_value: JsonValue) -> Result<RenameResult> {
        // 检查是否有错误
        if let Some(error_msg) = result_value.get("__error__").and_then(|v| v.as_str()) {
            anyhow::bail!("JS script error: {}", error_msg);
        }
        
        // 解析对象格式 {原始路径: 新路径}
        let obj = result_value.as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid result format: must be an object"))?;
        
        let mut mappings = Vec::new();
        
        for (original, new_name_value) in obj {
            let new_name = new_name_value.as_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid result format: new_name must be string"))?
                .to_string();
            
            if new_name.is_empty() {
                anyhow::bail!("New filename cannot be empty");
            }
            
            if new_name.len() > 255 {
                anyhow::bail!("New filename exceeds maximum length of 255 characters");
            }
            
            mappings.push((original.clone(), new_name));
        }
        
        // 如果只有一个映射，返回 Single；否则返回 Batch
        if mappings.len() == 1 {
            Ok(RenameResult::Single(mappings[0].1.clone()))
        } else {
            Ok(RenameResult::Batch(mappings))
        }
    }

    /// 获取收集的日志
    pub fn get_logs(&self) -> Vec<String> {
        self.log_collector.take_logs()
    }

    /// 执行重命名（对单个文件或文件夹）
    /// 如果返回批量结果，直接返回；如果返回单个结果，包装为批量结果
    /// 执行单个文件的重命名（内部辅助函数）
    pub fn execute_rename_single(
        &self,
        file_path: &str,
        script: &str,
        params: Option<&HashMap<String, JsonValue>>,
        options: ExecuteOptions,
    ) -> Result<Vec<(String, String)>> {
        let path = std::path::Path::new(file_path);
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| file_path.to_string());
        
        match self.execute_rename(&filename, script, Some(file_path), params, options) {
            Ok(RenameResult::Single(new_name)) => {
                // 检查是否是错误消息
                if new_name.starts_with("ERROR:") {
                    Ok(vec![(file_path.to_string(), new_name)])
                } else {
                    // 单个文件重命名
                    Ok(vec![(file_path.to_string(), new_name)])
                }
            }
            Ok(RenameResult::Batch(mappings)) => {
                // 批量重命名（文件夹场景）
                Ok(mappings)
            }
            Err(e) => {
                // 将错误转换为映射格式，以便前端显示
                Ok(vec![(file_path.to_string(), format!("ERROR: {}", e))])
            }
        }
    }
}

impl Default for JsEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create JS engine")
    }
}
