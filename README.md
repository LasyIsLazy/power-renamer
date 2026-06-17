# Power Renamer

一个基于 Tauri + Rust + Vue 的强大文件重命名工具，支持用户通过自定义 JavaScript 脚本实现重命名逻辑。

[![GitHub](https://img.shields.io/badge/GitHub-LasyIsLazy%2Fpower--renamer-blue)](https://github.com/LasyIsLazy/power-renamer)

## 功能特性

- 批量文件重命名
- 文件夹内文件批量重命名（递归）
- 自定义 JavaScript 重命名脚本
- 内置快速规则（转小写、加前缀、序号命名等）
- 实时预览重命名结果（路径差异高亮、树形展示）
- 目标路径冲突检测
- 撤销操作支持
- 历史记录持久化
- 文件名合法性验证
- 路径安全性检查

## 技术栈

- **前端**: Vue 3 + Pinia + Vite
- **后端**: Rust + Tauri 2.0
- **JS 引擎**: QuickJS (通过 rquickjs)

## 开发环境要求

- Node.js 18+
- Rust 1.70+
- Tauri CLI

## 安装依赖

```bash
npm install
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

### 基本流程

1. 选择文件或文件夹（支持拖放）
2. 选择脚本或快速规则
3. 设置脚本参数（如有）
4. 点击「预览」查看结果
5. 确认后「执行重命名」

### 快捷键

| 快捷键 | 操作 |
|--------|------|
| Ctrl+O | 选择文件 |
| Ctrl+Enter | 预览 |
| Ctrl+Shift+Enter | 执行重命名 |
| Ctrl+Z | 撤销 |
| Ctrl+R | 刷新脚本列表 |

### JavaScript 脚本格式

脚本必须定义无参数的 `rename()` 函数，并返回对象格式 `{原始路径: 新路径}`。

#### 可用全局变量

| 变量 | 说明 |
|------|------|
| `__filePath` | 当前文件/文件夹完整路径 |
| `__fileName` | 当前文件/文件夹名称 |
| `__params` | 脚本参数（在 manifest 中定义） |

#### 可用工具 API

- `__utils.path` — 路径操作（join、dirname、basename、extname 等）
- `__utils.md5` — MD5 哈希（file、string）
- `__utils.fs` — 文件系统（readDirFiles、readDirFilesRecursive）

#### 单文件示例

```javascript
function rename() {
  return { [__filePath]: __fileName.toLowerCase() };
}
```

#### 带参数示例

```javascript
function rename() {
  var prefix = __params && __params.prefix ? __params.prefix : 'IMG_';
  return { [__filePath]: prefix + __fileName };
}
```

#### 文件夹批量重命名示例

```javascript
function rename() {
  var result = {};
  var files = __utils.fs.readDirFilesRecursive(__filePath);
  for (var i = 0; i < files.length; i++) {
    result[files[i]] = 'new_' + i + __utils.path.extname(files[i]);
  }
  return result;
}
```

## 项目结构

```
power-renamer/
├── src/                    # Vue 前端
│   ├── components/         # Vue 组件
│   ├── stores/             # Pinia 状态管理
│   ├── composables/        # 可复用逻辑
│   └── constants/          # 快速规则等常量
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # Tauri 入口
│   │   ├── commands.rs     # Tauri 命令定义
│   │   └── js_engine.rs    # QuickJS 集成
│   └── Cargo.toml
└── package.json
```

## 贡献

欢迎提交 Issue 和 Pull Request！

GitHub: [https://github.com/LasyIsLazy/power-renamer](https://github.com/LasyIsLazy/power-renamer)

## 许可证

MIT
