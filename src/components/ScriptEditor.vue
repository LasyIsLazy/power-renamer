<template>
  <div class="script-editor">
    <div class="header">
      <h3>重命名脚本</h3>
      <div class="actions">
      </div>
    </div>

    <div class="editor-container">
      <textarea
        ref="editorRef"
        v-model="localScript"
        @input="handleInput"
        class="code-editor"
        placeholder="function rename() {&#10;  // 必须返回对象格式：{原始路径: 新路径}&#10;  // 单个文件示例：&#10;  return {[__filePath]: __fileName};&#10;  &#10;  // 批量重命名示例：&#10;  // var result = {};&#10;  // result[__filePath] = 'new_name.txt';&#10;  // return result;&#10;}"
      ></textarea>
    </div>

    <div class="footer">
      <div class="hint">
        <p>提示：脚本必须直接定义 <code>function rename()</code> 函数</p>
        <p>函数无参数，使用全局变量 <code>__filePath</code> 和 <code>__fileName</code></p>
        <p>使用 <code>__utils.path</code>, <code>__utils.md5</code>, <code>__utils.fs</code> 访问工具函数</p>
        <p>使用 <code>console.log()</code> 输出日志，日志会在预览面板中显示</p>
        <p><strong>必须返回对象格式：</strong><code>{原始路径: 新路径}</code></p>
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
