# Power Renamer

一个基于 Tauri + Rust + Vue 的强大文件重命名工具，支持用户通过自定义 JavaScript 脚本实现重命名逻辑。

[![GitHub](https://img.shields.io/badge/GitHub-LasyIsLazy%2Fpower--renamer-blue)](https://github.com/LasyIsLazy/power-renamer)

## 功能特性

- ✅ 批量文件重命名
- ✅ 文件夹内文件批量重命名
- ✅ 自定义 JavaScript 重命名脚本
- ✅ 实时预览重命名结果
- ✅ 撤销操作支持
- ✅ 历史记录查看
- ✅ 文件名合法性验证
- ✅ 路径安全性检查

## 技术栈

- **前端**: Vue 3 + Pinia + Vite
- **后端**: Rust + Tauri 2.0
- **JS 引擎**: QuickJS (通过 quickjs crate)

## 开发环境要求

- Node.js 18+
- Rust 1.70+
- Tauri CLI

## 安装依赖

```bash
# 安装前端依赖
npm install

# 安装 Tauri CLI (如果未安装)
npm install -g @tauri-apps/cli
```

## 开发运行

```bash
npm run tauri dev
```

## 构建应用

```bash
npm run tauri build
```

## 使用说明

### JavaScript 脚本格式

脚本必须定义一个 `rename(filename)` 函数，接收文件名（字符串），返回新的文件名（字符串）。

示例：

```javascript
function rename(filename) {
  // 转换为小写
  return filename.toLowerCase();
}
```

```javascript
function rename(filename) {
  // 移除空格并替换为下划线
  return filename.replace(/\s+/g, '_');
}
```

```javascript
function rename(filename) {
  // 添加时间戳前缀
  const ext = filename.substring(filename.lastIndexOf('.'));
  const name = filename.substring(0, filename.lastIndexOf('.'));
  return 'IMG_' + Date.now() + '_' + name + ext;
}
```

## 项目结构

```
power-renamer/
├── src/                    # Vue 前端
│   ├── components/         # Vue 组件
│   ├── stores/            # Pinia 状态管理
│   └── main.js
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── main.rs        # Tauri 入口
│   │   ├── commands.rs    # Tauri 命令定义
│   │   └── js_engine.rs   # QuickJS 集成
│   └── Cargo.toml
└── package.json
```

## 贡献

欢迎提交 Issue 和 Pull Request！

GitHub: [https://github.com/LasyIsLazy/power-renamer](https://github.com/LasyIsLazy/power-renamer)

## 许可证

MIT
