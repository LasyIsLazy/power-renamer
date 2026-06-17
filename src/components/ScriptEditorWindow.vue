<template>
  <div class="script-editor-window">
    <div class="header">
      <h3>{{ scriptId ? '编辑脚本' : '新建脚本' }}</h3>
      <div class="actions">
        <button @click="loadTemplate" class="btn btn-secondary">加载示例</button>
        <button @click="formatCode" class="btn btn-secondary">格式化</button>
        <button @click="saveScript" class="btn btn-primary">保存</button>
        <button @click="closeWindow" class="btn btn-secondary">关闭</button>
      </div>
    </div>

    <div class="editor-container">
      <div v-if="scriptNameEditable" class="script-name-input">
        <input
          v-model="scriptName"
          type="text"
          placeholder="脚本名称"
          class="name-input"
        />
      </div>
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
        <p><strong>必须返回对象格式：</strong><code>{原始路径: 新路径}</code></p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRenameStore } from '../stores/renameStore'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { emit, listen } from '@tauri-apps/api/event'

const store = useRenameStore()
const editorRef = ref(null)
const localScript = ref('')
const scriptName = ref('')
const scriptId = ref(null)
const scriptNameEditable = ref(true)

onMounted(async () => {
  if (store.savedScripts.length === 0) {
    await store.initSavedScripts()
  }

  const urlParams = new URLSearchParams(window.location.search)
  const id = urlParams.get('id')

  if (id) {
    await loadScriptById(id)
  } else {
    localScript.value = store.script
  }

  await listen('load-script', async (event) => {
    if (event.payload?.scriptId) {
      await loadScriptById(event.payload.scriptId)
    }
  })
})

async function loadScriptById(id) {
  scriptId.value = id
  await store.initSavedScripts()
  const script = store.savedScripts.find((s) => s.id === id)
  if (script) {
    localScript.value = script.script
    scriptName.value = script.name
    scriptNameEditable.value = false
  }
}

const handleInput = () => {
  // 编辑器窗口独立状态，不污染主窗口 store
}

const loadTemplate = () => {
  const templates = [
    `function rename() {
  // 转换为小写
  var newName = __fileName.toLowerCase();
  return {[__filePath]: newName};
}`,
    `function rename() {
  // 移除空格
  var newName = __fileName.replace(/\\s+/g, '_');
  return {[__filePath]: newName};
}`,
    `function rename() {
  // 添加前缀
  var newName = 'IMG_' + __fileName;
  return {[__filePath]: newName};
}`,
    `function rename() {
  // 按 MD5 重命名
  if (__utils.path.isFile(__filePath)) {
    var ext = __utils.path.extname(__filePath);
    var hash = __utils.md5.file(__filePath);
    return {[__filePath]: hash + ext};
  }
  return {[__filePath]: __fileName};
}`,
    `function rename() {
  // 文件夹批量重命名示例
  if (__utils.path.isDir(__filePath)) {
    var files = __utils.fs.readDirFilesRecursive(__filePath);
    var result = {};
    for (var i = 0; i < files.length; i++) {
      var filePath = files[i];
      if (__utils.path.isFile(filePath)) {
        var ext = __utils.path.extname(filePath);
        var hash = __utils.md5.file(filePath);
        result[filePath] = hash + ext;
      }
    }
    return result;
  }
  return {[__filePath]: __fileName};
}`,
  ]

  const randomTemplate = templates[Math.floor(Math.random() * templates.length)]
  localScript.value = randomTemplate
}

const formatCode = () => {
  // 简单的代码格式化（实际项目中可以使用 prettier 等工具）
  try {
    const formatted = localScript.value
      .split('\n')
      .map(line => line.trimEnd())
      .join('\n')
      .replace(/\n{3,}/g, '\n\n')
    
    localScript.value = formatted
  } catch (error) {
    console.error('Format error:', error)
  }
}

const saveScript = async () => {
  if (!scriptName.value.trim()) {
    alert('请输入脚本名称')
    return
  }
  
  if (!localScript.value.trim()) {
    alert('脚本内容不能为空')
    return
  }

  const success = await store.saveScript(scriptName.value.trim(), localScript.value, scriptId.value)
  if (success) {
    const savedId = scriptId.value || scriptName.value.trim().replace(/[<>:"/\\|?*]/g, '_')
    await emit('script-saved', { scriptId: savedId })
    await emit('scripts-changed')
    if (!scriptId.value) {
      closeWindow()
    }
  }
}

const closeWindow = async () => {
  try {
    const currentWindow = getCurrentWindow()
    await currentWindow.close()
  } catch (error) {
    console.error('Failed to close window:', error)
  }
}
</script>

<style scoped>
.script-editor-window {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #f5f5f5;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
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

.btn-primary {
  background: #007bff;
  color: white;
}

.btn-primary:hover {
  background: #0056b3;
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
  display: flex;
  flex-direction: column;
  padding: 16px;
  min-height: 0;
}

.script-name-input {
  margin-bottom: 12px;
}

.name-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  font-size: 14px;
}

.code-editor {
  flex: 1;
  width: 100%;
  padding: 12px;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  font-family: 'Courier New', Monaco, monospace;
  font-size: 14px;
  line-height: 1.5;
  resize: none;
  background: #fff;
}

.code-editor:focus {
  outline: none;
  border-color: #007bff;
  box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
}

.footer {
  padding: 12px 16px;
  background: white;
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
  background: #f5f5f5;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'Courier New', Monaco, monospace;
}
</style>
