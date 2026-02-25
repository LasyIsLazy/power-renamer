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
        placeholder="function rename(filename) {&#10;  // 在这里编写你的重命名逻辑&#10;  return filename;&#10;}"
      ></textarea>
    </div>

    <div class="footer">
      <div class="hint">
        <p>提示：脚本必须定义一个 <code>rename(filename)</code> 函数</p>
        <p>函数接收文件名（字符串），返回新的文件名（字符串）</p>
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
    `function rename(filename) {
  // 转换为小写
  return filename.toLowerCase();
}`,
    `function rename(filename) {
  // 移除空格
  return filename.replace(/\\s+/g, '_');
}`,
    `function rename(filename) {
  // 添加前缀
  return 'IMG_' + filename;
}`,
    `function rename(filename) {
  // 添加序号
  const ext = filename.substring(filename.lastIndexOf('.'));
  const name = filename.substring(0, filename.lastIndexOf('.'));
  return name + '_' + Date.now() + ext;
}`,
    `function rename(filename) {
  // 替换特定字符
  return filename.replace(/[^a-zA-Z0-9._-]/g, '_');
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
