use anyhow::{Context, Result};
use rquickjs::{Context as JsContext, Runtime as JsRuntime};
use std::sync::{Arc, Mutex};

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

    /// 执行重命名脚本，对单个文件名应用重命名逻辑
    pub fn execute_rename(&self, filename: &str, script: &str) -> Result<String> {
        if script.trim().is_empty() {
            anyhow::bail!("Script is empty");
        }

        let runtime = self.runtime.lock().unwrap();
        let context = JsContext::full(&runtime)
            .context("Failed to create JS context")?;

        // 转义文件名中的特殊字符
        let escaped_filename = filename
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        // QuickJS-NG 完全支持 ES6 语法，包括 export default
        // 将 export default 转换为可执行的代码，以便在 Rust 中访问导出的函数
        let module_script = if script.contains("export default function") {
            // export default function rename(...) { ... }
            script.replace("export default function", "function")
        } else {
            // export default (其他形式，如箭头函数或已定义的函数)
            script.replace("export default", "var __default_export =")
        };
        
        let wrapped_script = format!(
            r#"
            (function() {{
                {}
                
                // 获取默认导出
                var rename = typeof __default_export !== 'undefined' ? __default_export : rename;
                
                // 确保 rename 函数存在
                if (typeof rename !== 'function') {{
                    throw new Error('export default must export a function');
                }}
                
                // 执行重命名
                var result = rename("{}");
                
                // 确保返回字符串
                if (typeof result !== 'string') {{
                    throw new Error('rename function must return a string');
                }}
                
                return result;
            }})()
            "#,
            module_script,
            escaped_filename
        );

        // 执行脚本
        let result = context.with(|ctx| {
            let result: String = ctx.eval(wrapped_script.as_str())
                .context("Failed to execute JS script")?;
            
            // 验证结果
            if result.is_empty() {
                anyhow::bail!("Rename function returned empty string");
            }

            // 限制文件名长度
            if result.len() > 255 {
                anyhow::bail!("New filename exceeds maximum length of 255 characters");
            }

            Ok(result)
        })?;

        Ok(result)
    }

    /// 批量执行重命名，返回原文件名到新文件名的映射
    pub fn execute_rename_batch(
        &self,
        filenames: &[String],
        script: &str,
    ) -> Result<Vec<(String, String)>> {
        let mut results = Vec::new();

        for filename in filenames {
            match self.execute_rename(filename, script) {
                Ok(new_name) => results.push((filename.clone(), new_name)),
                Err(e) => {
                    // 如果单个文件失败，记录错误但继续处理其他文件
                    eprintln!("Error renaming {}: {}", filename, e);
                    results.push((filename.clone(), format!("ERROR: {}", e)));
                }
            }
        }

        Ok(results)
    }
}

impl Default for JsEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create JS engine")
    }
}
