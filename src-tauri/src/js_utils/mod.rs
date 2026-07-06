use anyhow::Result;
use rquickjs::{Ctx, Function, Object};
use std::sync::{Arc, Mutex};

mod path;
pub mod md5;
mod fs;

pub use md5::{Md5Options, Md5PreviewMode};

/// 日志收集器：用于收集 console.log 的输出
#[derive(Clone)]
pub struct LogCollector {
    logs: Arc<Mutex<Vec<String>>>,
}

impl LogCollector {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_log(&self, message: String) {
        eprintln!("[LOG] Adding log message: {}", message);
        if let Ok(mut logs) = self.logs.lock() {
            logs.push(message);
            eprintln!("[LOG] Log added successfully. Total logs: {}", logs.len());
        } else {
            eprintln!("[ERROR] Failed to lock logs mutex");
        }
    }

    pub fn take_logs(&self) -> Vec<String> {
        if let Ok(mut logs) = self.logs.lock() {
            std::mem::take(&mut *logs)
        } else {
            Vec::new()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.clear();
        }
    }
}

/// 设置 JS 工具函数，将所有工具函数统一放到 __utils 全局对象中
/// 包括 path、md5、fs、log 等工具函数
pub fn setup_js_utils<'js>(
    ctx: &Ctx<'js>,
    log_collector: Option<&LogCollector>,
    md5_options: Md5Options,
) -> Result<()> {
    let global = ctx.globals();

    // 创建 __utils 对象
    let utils_obj = Object::new(ctx.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create __utils object: {}", e))?;

    // 创建 path 对象
    let path_obj = Object::new(ctx.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create path object: {}", e))?;
    
    // 绑定 path.join - 支持两个参数（最常用的情况）
    let join_fn = Function::new(ctx.clone(), path::join2)
        .map_err(|e| anyhow::anyhow!("Failed to create path.join function: {}", e))?;
    path_obj.set("join", join_fn)
        .map_err(|e| anyhow::anyhow!("Failed to set path.join: {}", e))?;
    
    // 绑定 path.dirname
    let dirname_fn = Function::new(ctx.clone(), path::dirname)?;
    path_obj.set("dirname", dirname_fn)?;
    
    // 绑定 path.basename
    let basename_fn = Function::new(ctx.clone(), path::basename)?;
    path_obj.set("basename", basename_fn)?;
    
    // 绑定 path.extname
    let extname_fn = Function::new(ctx.clone(), path::extname)?;
    path_obj.set("extname", extname_fn)?;
    
    // 绑定 path.normalize
    let normalize_fn = Function::new(ctx.clone(), path::normalize)?;
    path_obj.set("normalize", normalize_fn)?;
    
    // 绑定 path.isAbsolute
    let is_absolute_fn = Function::new(ctx.clone(), path::is_absolute)?;
    path_obj.set("isAbsolute", is_absolute_fn)?;
    
    // 绑定 path.sep
    let sep_fn = Function::new(ctx.clone(), path::sep)?;
    path_obj.set("sep", sep_fn)?;
    
    // 绑定 path.isFile
    let is_file_fn = Function::new(ctx.clone(), path::is_file)?;
    path_obj.set("isFile", is_file_fn)?;
    
    // 绑定 path.isDir
    let is_dir_fn = Function::new(ctx.clone(), path::is_dir)?;
    path_obj.set("isDir", is_dir_fn)?;
    
    // 绑定 path.exists
    let exists_fn = Function::new(ctx.clone(), path::exists)?;
    path_obj.set("exists", exists_fn)?;
    
    // 将 path 对象添加到 __utils
    utils_obj.set("path", path_obj)?;

    // 创建 md5 对象
    let md5_obj = Object::new(ctx.clone())?;
    
    // 绑定 md5.file
    let file_md5_fn = Function::new(ctx.clone(), move |file_path: String| -> String {
        md5::file_md5_with_options(file_path, &md5_options)
    })?;
    md5_obj.set("file", file_md5_fn)?;
    
    // 绑定 md5.string
    let string_md5_fn = Function::new(ctx.clone(), md5::string_md5)?;
    md5_obj.set("string", string_md5_fn)?;
    
    // 将 md5 对象添加到 __utils
    utils_obj.set("md5", md5_obj)?;

    // 创建 fs 对象
    let fs_obj = Object::new(ctx.clone())?;
    
    // 绑定 fs.readDirFiles（仅读取当前目录的文件，不包括子文件夹）
    let read_dir_files_fn = Function::new(ctx.clone(), |dir_path: String| -> Vec<String> {
        fs::read_dir_files(dir_path).unwrap_or_else(|e| {
            eprintln!("Error reading directory: {}", e);
            Vec::new()
        })
    })?;
    fs_obj.set("readDirFiles", read_dir_files_fn)?;
    
    // 绑定 fs.readDirFilesRecursive（递归读取所有文件）
    let read_dir_files_recursive_fn = Function::new(ctx.clone(), |dir_path: String| -> Vec<String> {
        fs::read_dir_files_recursive(dir_path).unwrap_or_else(|e| {
            eprintln!("Error reading directory recursively: {}", e);
            Vec::new()
        })
    })?;
    fs_obj.set("readDirFilesRecursive", read_dir_files_recursive_fn)?;
    
    // 将 fs 对象添加到 __utils
    utils_obj.set("fs", fs_obj)?;

    // 劫持 console.log 以收集日志
    if let Some(collector) = log_collector {
        eprintln!("[DEBUG] Setting up console.log hijacking...");
        
        let collector_clone = collector.clone();
        // 创建一个 Rust 函数来接收字符串消息
        let log_handler = Function::new(ctx.clone(), move |msg: String| -> () {
            collector_clone.add_log(msg);
        })
        .map_err(|e| {
            eprintln!("[ERROR] Failed to create log handler: {}", e);
            anyhow::anyhow!("Failed to create log handler function: {}", e)
        })?;
        
        eprintln!("[DEBUG] Log handler created successfully");
        
        // 将 handler 设置到全局，供 JS 使用
        ctx.globals().set("__log_handler_temp", log_handler)
            .map_err(|e| {
                eprintln!("[ERROR] Failed to set __log_handler_temp: {}", e);
                anyhow::anyhow!("Failed to set __log_handler_temp: {}", e)
            })?;
        
        eprintln!("[DEBUG] __log_handler_temp set to global scope");
        
        // 劫持 console.log，将日志转发到收集器
        // 先确保 console 对象存在
        let hijack_console_log_script = r#"
            (function() {
                try {
                    // 确保 console 对象存在
                    if (typeof console === 'undefined') {
                        globalThis.console = {};
                    }
                    if (typeof console.log === 'undefined') {
                        console.log = function() {};
                    }
                    
                    var handler = globalThis.__log_handler_temp;
                    if (typeof handler !== 'function') {
                        throw new Error('Log handler is not a function');
                    }
                    
                    var originalConsoleLog = console.log;
                    
                    console.log = function() {
                        try {
                            // 将参数转换为字符串并发送到收集器
                            var args = Array.prototype.slice.call(arguments);
                            var message = args.map(function(arg) {
                                if (typeof arg === 'object') {
                                    try {
                                        return JSON.stringify(arg);
                                    } catch (e) {
                                        return String(arg);
                                    }
                                }
                                return String(arg);
                            }).join(' ');
                            // 调用 handler 收集日志
                            handler(message);
                        } catch (e) {
                            // 如果收集日志失败，至少尝试调用原始 console.log
                            if (typeof originalConsoleLog === 'function') {
                                originalConsoleLog.apply(console, arguments);
                            }
                        }
                    };
                } catch (e) {
                    throw new Error('Failed to hijack console.log: ' + (e.message || String(e)));
                }
            })();
        "#;
        ctx.eval::<(), _>(hijack_console_log_script)
            .map_err(|e| anyhow::anyhow!("Failed to hijack console.log: {}. This may be due to QuickJS console object issues.", e))?;
    }

    // 将 __utils 设置到全局作用域
    eprintln!("[DEBUG] Setting __utils to global scope...");
    global.set("__utils", utils_obj)
        .map_err(|e| {
            eprintln!("[ERROR] Failed to set __utils to global scope: {}", e);
            anyhow::anyhow!("Failed to set __utils to global scope: {}", e)
        })?;
    
    eprintln!("[DEBUG] JS utils setup completed successfully");
    Ok(())
}
