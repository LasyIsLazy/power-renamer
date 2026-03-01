use anyhow::Result;
use rquickjs::{Ctx, Function, Object};

mod path;
mod md5;
mod fs;

/// 设置 JS 工具函数，将所有工具函数统一放到 __utils 全局对象中
pub fn setup_js_utils<'js>(ctx: &Ctx<'js>) -> Result<()> {
    let global = ctx.globals();

    // 创建 __utils 对象
    let utils_obj = Object::new(ctx.clone())?;

    // 创建 path 对象
    let path_obj = Object::new(ctx.clone())?;
    
    // 绑定 path.join - 支持两个参数（最常用的情况）
    let join_fn = Function::new(ctx.clone(), path::join2)?;
    path_obj.set("join", join_fn)?;
    
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
    let file_md5_fn = Function::new(ctx.clone(), |file_path: String| -> String {
        md5::file_md5(file_path).unwrap_or_else(|e| {
            // 如果出错，返回错误信息（或者可以抛出 JS 异常）
            format!("ERROR: {}", e)
        })
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

    // 将 __utils 设置到全局作用域
    global.set("__utils", utils_obj)?;

    Ok(())
}
