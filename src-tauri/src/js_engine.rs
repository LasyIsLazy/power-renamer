use anyhow::{Context, Result};
use rquickjs::{Context as JsContext, Runtime as JsRuntime};
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};

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
}

unsafe impl Send for JsEngine {}
unsafe impl Sync for JsEngine {}

impl JsEngine {
    pub fn new() -> Result<Self> {
        let runtime = JsRuntime::new().context("Failed to create JS runtime")?;
        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
        })
    }

    /// 执行重命名脚本
    /// file_path: 文件或文件夹的完整路径
    /// filename: 文件或文件夹名称
    /// 返回：单个文件的新名称，或批量重命名映射
    pub fn execute_rename(&self, filename: &str, script: &str, file_path: Option<&str>) -> Result<RenameResult> {
        if script.trim().is_empty() {
            anyhow::bail!("Script is empty");
        }

        let runtime = self.runtime.lock().unwrap();
        let context = JsContext::full(&runtime)
            .context("Failed to create JS context")?;

        // 执行脚本
        let result = context.with(|ctx| {
            // 1. 首先注入 JS 工具函数到 __utils 对象（path、md5、fs）
            // 这些工具函数需要在编译脚本之前就注入，这样脚本编译时就能访问到
            crate::js_utils::setup_js_utils(&ctx)
                .context("Failed to setup JS utils")?;
            
            // 2. 编译脚本（用户脚本直接定义 rename 函数）
            // 此时脚本已经可以访问 __utils.path、__utils.md5、__utils.fs 等工具函数
            ctx.eval::<(), _>(script)
                .context("Failed to compile JS script")?;
            
            // 3. 设置执行时的全局变量（每次调用可能不同）
            if let Some(path) = file_path {
                ctx.globals().set("__filePath", path)
                    .context("Failed to set __filePath")?;
            } else {
                // 使用 null 字符串，在 JS 端会被解析为 null
                ctx.eval::<(), _>("var __filePath = null;")
                    .context("Failed to set __filePath")?;
            }
            ctx.globals().set("__fileName", filename)
                .context("Failed to set __fileName")?;
            
            // 4. 调用 rename 函数并序列化结果
            // 由于 rquickjs 的 Value 类型转换复杂，我们在 JS 端处理返回值
            let json_str: String = ctx.eval(
                r#"
                (function() {
                    try {
                        var result = rename();
                        if (typeof result === 'string') {
                            return JSON.stringify({type: 'single', value: result});
                        } else if (Array.isArray(result)) {
                            return JSON.stringify({type: 'batch', value: result});
                        } else if (typeof result === 'object' && result !== null) {
                            var batch = [];
                            for (var key in result) {
                                if (result.hasOwnProperty(key)) {
                                    batch.push([key, result[key]]);
                                }
                            }
                            return JSON.stringify({type: 'batch', value: batch});
                        } else {
                            throw new Error('rename function must return a string, object, or array');
                        }
                    } catch (error) {
                        var errorMsg = error.message || String(error);
                        if (error.stack) {
                            errorMsg += '\nStack: ' + error.stack;
                        }
                        return JSON.stringify({type: 'error', value: errorMsg});
                    }
                })()
                "#
            )
            .context("Failed to execute rename function")?;
            
            // 5. 解析 JSON
            let result_value: JsonValue = serde_json::from_str(&json_str)
                .context("Failed to parse JS result as JSON")?;
            
            // 6. 处理结果
            self.parse_rename_result_from_json(result_value)
        })?;

        Ok(result)
    }
    
    /// 从 JSON 解析重命名函数的返回值
    fn parse_rename_result_from_json(&self, result_value: JsonValue) -> Result<RenameResult> {
        // 检查是否有错误
        if let Some(error_type) = result_value.get("type").and_then(|v| v.as_str()) {
            if error_type == "error" {
                let error_msg = result_value.get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("JS script error: {}", error_msg);
            }
        }
        
        let result_type = result_value.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid result format: missing type"))?;
        
        match result_type {
            "single" => {
                let value = result_value.get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Invalid result format: missing value"))?;
                
                if value.is_empty() {
                    anyhow::bail!("Rename function returned empty string");
                }
                
                if value.len() > 255 {
                    anyhow::bail!("New filename exceeds maximum length of 255 characters");
                }
                
                Ok(RenameResult::Single(value.to_string()))
            }
            "batch" => {
                let value = result_value.get("value")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("Invalid result format: batch value must be array"))?;
                
                let mut mappings = Vec::new();
                for item in value {
                    if let Some(arr) = item.as_array() {
                        if arr.len() >= 2 {
                            let original = arr[0].as_str()
                                .ok_or_else(|| anyhow::anyhow!("Invalid batch format: original must be string"))?
                                .to_string();
                            let new_name = arr[1].as_str()
                                .ok_or_else(|| anyhow::anyhow!("Invalid batch format: new_name must be string"))?
                                .to_string();
                            
                            if new_name.is_empty() {
                                anyhow::bail!("New filename cannot be empty");
                            }
                            
                            if new_name.len() > 255 {
                                anyhow::bail!("New filename exceeds maximum length of 255 characters");
                            }
                            
                            mappings.push((original, new_name));
                        }
                    }
                }
                
                Ok(RenameResult::Batch(mappings))
            }
            _ => anyhow::bail!("Invalid result type: {}", result_type),
        }
    }

    /// 执行重命名（对单个文件或文件夹）
    /// 如果返回批量结果，直接返回；如果返回单个结果，包装为批量结果
    pub fn execute_rename_single(
        &self,
        file_path: &str,
        script: &str,
    ) -> Result<Vec<(String, String)>> {
        let path = std::path::Path::new(file_path);
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| file_path.to_string());
        
        match self.execute_rename(&filename, script, Some(file_path)) {
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
