<template>
  <div class="script-editor">
    <div class="header">
      <h3>重命名脚本</h3>
      <div class="actions">
        <button @click="loadTemplate" class="btn btn-secondary">加载示例</button>
        <button @click="formatCode" class="btn btn-secondary">格式化</button>
      </div>
    </div>

    <div class="editor-container">
      <textarea
        ref="editorRef"
        v-model="localScript"
        @input="handleInput"
        class="code-editor"
        placeholder="function rename() {&#10;  // 使用全局变量 __filePath 和 __fileName&#10;  // 使用 __utils.path, __utils.md5, __utils.fs 访问工具函数&#10;  // 返回字符串（单个文件）或对象/数组（批量重命名）&#10;  return __fileName;&#10;}"
      ></textarea>
    </div>

    <div class="footer">
      <div class="hint">
        <p>提示：脚本必须直接定义 <code>function rename()</code> 函数</p>
        <p>函数无参数，使用全局变量 <code>__filePath</code> 和 <code>__fileName</code></p>
        <p>使用 <code>__utils.path</code>, <code>__utils.md5</code>, <code>__utils.fs</code> 访问工具函数</p>
        <p>返回字符串（单个文件）或对象/数组（批量重命名，用于文件夹场景）</p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, onMounted } from 'vue'
import { useRenameStore } from '../stores/renameStore'

const store = useRenameStore()
const editorRef = ref(null)
const localScript = ref(store.script)

// 监听 store 中的脚本变化
watch(
  () => store.script,
  (newScript) => {
    if (newScript !== localScript.value) {
      localScript.value = newScript
    }
  }
)

const handleInput = () => {
  store.updateScript(localScript.value)
}

const loadTemplate = () => {
  const templates = [
    `function rename() {
  // 转换为小写
  var dir = __utils.path.dirname(__filePath);
  var newName = __fileName.toLowerCase();
  return __utils.path.join(dir, newName);
}`,
    `function rename() {
  // 移除空格
  var dir = __utils.path.dirname(__filePath);
  var newName = __fileName.replace(/\\s+/g, '_');
  return __utils.path.join(dir, newName);
}`,
    `function rename() {
  // 添加前缀
  var dir = __utils.path.dirname(__filePath);
  var newName = 'IMG_' + __fileName;
  return __utils.path.join(dir, newName);
}`,
    `function rename() {
  // 按 MD5 重命名
  if (__utils.path.isFile(__filePath)) {
    var ext = __utils.path.extname(__filePath);
    var hash = __utils.md5.file(__filePath);
    var dir = __utils.path.dirname(__filePath);
    return __utils.path.join(dir, hash + ext);
  }
  return __fileName;
}`,
    `function rename() {
  // 文件夹批量重命名示例
  if (__utils.path.isDir(__filePath)) {
    var files = __utils.fs.readDirFiles(__filePath);
    var result = {};
    for (var i = 0; i < files.length; i++) {
      var filePath = files[i];
      if (__utils.path.isFile(filePath)) {
        var ext = __utils.path.extname(filePath);
        var hash = __utils.md5.file(filePath);
        var dir = __utils.path.dirname(filePath);
        result[filePath] = __utils.path.join(dir, hash + ext);
      }
    }
    return result;
  }
  return __fileName;
}`,
  ]

  const randomTemplate = templates[Math.floor(Math.random() * templates.length)]
  localScript.value = randomTemplate
  store.updateScript(randomTemplate)
}

const formatCode = () => {
  // 简单的代码格式化（实际项目中可以使用 prettier 等工具）
  try {
    const formatted = localScript.value
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.length > 0)
      .join('\n')
    localScript.value = formatted
    store.updateScript(formatted)
  } catch (error) {
    console.error('格式化失败:', error)
  }
}

onMounted(() => {
  // 设置编辑器样式
  if (editorRef.value) {
    editorRef.value.style.fontFamily = 'Monaco, "Courier New", monospace'
    editorRef.value.style.fontSize = '14px'
  }
})
</script>

<style scoped>
.script-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow: hidden;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: #f5f5f5;
  border-bottom: 1px solid #e0e0e0;
}

.header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.actions {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 6px 12px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  transition: background-color 0.2s;
}

.btn-secondary {
  background: #6c757d;
  color: white;
}

.btn-secondary:hover {
  background: #545b62;
}

.editor-container {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.code-editor {
  width: 100%;
  height: 100%;
  padding: 16px;
  border: none;
  resize: none;
  font-family: Monaco, 'Courier New', monospace;
  font-size: 14px;
  line-height: 1.6;
  background: #fff;
  color: #333;
  tab-size: 2;
}

.code-editor:focus {
  outline: none;
}

.footer {
  padding: 12px 16px;
  background: #f9f9f9;
  border-top: 1px solid #e0e0e0;
}

.hint {
  font-size: 12px;
  color: #666;
}

.hint p {
  margin: 4px 0;
}

.hint code {
  background: #e9ecef;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: Monaco, 'Courier New', monospace;
  font-size: 11px;
  color: #d63384;
}
</style>
