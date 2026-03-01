<template>
  <div class="script-list">
    <div class="header">
      <h3>脚本列表</h3>
      <div class="actions">
        <button @click="openEditor" class="btn btn-primary">新建脚本</button>
        <button @click="reloadScripts" class="btn btn-secondary" title="重新加载脚本列表">🔄</button>
        <button @click="openScriptsDir" class="btn btn-secondary">打开脚本目录</button>
      </div>
    </div>

    <div v-if="store.currentScriptId" class="current-script-banner">
      <span class="current-script-label">当前使用:</span>
      <span class="current-script-name">{{ currentScriptName }}</span>
    </div>

    <div class="script-items">
      <div
        v-for="script in store.savedScripts"
        :key="script.id"
        class="script-item"
        :class="{ active: store.currentScriptId === script.id }"
        @click="selectScript(script.id)"
      >
        <div class="script-info">
          <div class="script-name">
            {{ script.display_name || script.name }}
          </div>
          <div v-if="script.description" class="script-description">
            {{ script.description }}
          </div>
          <div class="script-meta">
            <span class="script-file-name">文件名: {{ script.name }}</span>
            <span>更新于: {{ formatDate(script.updated_at) }}</span>
          </div>
        </div>
        <div class="script-actions" @click.stop>
          <button
            @click="useScript(script.id)"
            class="btn-icon"
            title="使用此脚本"
          >
            ✓
          </button>
          <button
            @click="renameScript(script.id, script.name)"
            class="btn-icon"
            title="重命名"
          >
            📝
          </button>
          <button
            @click="editScript(script.id)"
            class="btn-icon"
            title="编辑"
          >
            ✏️
          </button>
          <button
            @click="editManifest(script.id)"
            class="btn-icon"
            title="编辑信息"
          >
            📋
          </button>
          <button
            @click="deleteScript(script.id)"
            class="btn-icon"
            title="删除"
          >
            🗑️
          </button>
        </div>
      </div>

      <div v-if="store.savedScripts.length === 0" class="empty">
        <p>暂无保存的脚本</p>
        <button @click="openEditor" class="btn btn-primary">创建第一个脚本</button>
      </div>
    </div>

  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRenameStore } from '../stores/renameStore'

async function getInvoke() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke
  } catch (error) {
    console.error('Failed to import invoke:', error)
    throw new Error('Tauri API 不可用，请在 Tauri 应用中运行')
  }
}

const store = useRenameStore()
const currentScriptId = ref(null)

const currentScriptName = computed(() => {
  if (store.currentScriptId) {
    const script = store.savedScripts.find((s) => s.id === store.currentScriptId)
    return script ? script.name : store.currentScriptId
  }
  return ''
})

onMounted(async () => {
  await store.initSavedScripts()
})

const formatDate = (dateString) => {
  const date = new Date(dateString)
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const selectScript = async (scriptId) => {
  // 点击脚本时直接使用
  await useScript(scriptId)
}

const useScript = async (scriptId) => {
  if (scriptId) {
    await store.loadScript(scriptId)
  } else if (currentScriptId.value) {
    // 兼容旧的调用方式
    await store.loadScript(currentScriptId.value)
    currentScriptId.value = null
  }
}

const editScript = async (scriptId) => {
  try {
    const invoke = await getInvoke()
    await invoke('open_script_file', { scriptId })
  } catch (error) {
    console.error('Failed to open script file:', error)
    store.error = `打开脚本文件失败: ${error}`
  }
}

const openEditor = async () => {
  // 新建脚本：创建临时文件并打开
  try {
    const invoke = await getInvoke()
    const scriptsDir = await invoke('get_scripts_dir_display')
    const tempName = `new_script_${Date.now()}`
    const tempScript = `function rename() {
  // 必须返回对象格式：{原始路径: 新路径}
  return {[__filePath]: __fileName};
}`
    
    // 先保存临时文件
    await invoke('save_script', {
      name: tempName,
      script: tempScript,
      scriptId: null
    })
    
    // 然后打开文件
    await invoke('open_script_file', { scriptId: tempName })
    
    // 刷新脚本列表
    await store.initSavedScripts()
  } catch (error) {
    console.error('Failed to create and open new script:', error)
    store.error = `创建新脚本失败: ${error}`
  }
}

const deleteScript = async (scriptId) => {
  if (confirm('确定要删除这个脚本吗？')) {
    await store.deleteScript(scriptId)
    if (currentScriptId.value === scriptId) {
      currentScriptId.value = null
    }
  }
}

const reloadScripts = async () => {
  try {
    await store.initSavedScripts()
  } catch (error) {
    console.error('Failed to reload scripts:', error)
    store.error = `重新加载脚本失败: ${error}`
  }
}

const openScriptsDir = async () => {
  try {
    const invoke = await getInvoke()
    // 使用特殊 ID "." 表示打开目录
    await invoke('open_script_file', { scriptId: '.' })
  } catch (error) {
    console.error('Failed to open scripts directory:', error)
    store.error = `打开脚本目录失败: ${error}`
  }
}

const editManifest = async (scriptId) => {
  try {
    const invoke = await getInvoke()
    // 直接打开 manifest 文件
    await invoke('open_script_manifest', { scriptId })
    // 延迟刷新，等待用户编辑完成
    setTimeout(async () => {
      await store.initSavedScripts()
    }, 1000)
  } catch (error) {
    console.error('Failed to open manifest file:', error)
    store.error = `打开脚本信息文件失败: ${error}`
  }
}

const renameScript = async (scriptId, currentName) => {
  const newName = prompt('请输入新的脚本名称:', currentName)
  if (newName && newName.trim() && newName.trim() !== currentName) {
    try {
      const success = await store.renameScript(scriptId, newName.trim())
      if (success) {
        // 刷新脚本列表
        await store.initSavedScripts()
        // 如果当前脚本被重命名，更新 currentScriptId
        // 脚本 ID 是基于文件名的（清理后的文件名），重命名后 ID 会改变
        if (store.currentScriptId === scriptId) {
          // 需要找到新的脚本 ID（基于清理后的新名称）
          // 清理文件名，移除非法字符（与后端逻辑一致）
          const safeName = newName.trim()
            .replace(/[<>:"/\\|?*]/g, '_')
          const newScript = store.savedScripts.find(s => s.id === safeName || s.name === newName.trim())
          if (newScript) {
            await store.loadScript(newScript.id)
          } else {
            // 如果找不到，尝试使用清理后的名称作为 ID
            await store.loadScript(safeName)
          }
        }
      }
    } catch (error) {
      console.error('Failed to rename script:', error)
      store.error = `重命名脚本失败: ${error}`
    }
  }
}
</script>

<style scoped>
.script-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow: hidden;
  background: white;
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

.current-script-banner {
  padding: 8px 16px;
  background: #e3f2fd;
  border-bottom: 1px solid #90caf9;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.current-script-label {
  color: #1976d2;
  font-weight: 500;
}

.current-script-name {
  color: #1565c0;
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
  background: #5a6268;
}

.btn-success {
  background: #28a745;
  color: white;
}

.btn-success:hover {
  background: #218838;
}

.script-items {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.script-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  margin-bottom: 8px;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.script-item:hover {
  border-color: #007bff;
  background: #f8f9fa;
}

.script-item.active {
  border-color: #007bff;
  background: #e7f3ff;
}

.script-info {
  flex: 1;
}

.script-name {
  font-weight: 500;
  color: #333;
  margin-bottom: 4px;
}

.script-description {
  font-size: 13px;
  color: #666;
  margin-bottom: 4px;
  line-height: 1.4;
}

.script-meta {
  font-size: 12px;
  color: #666;
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.script-file-name {
  font-family: Monaco, 'Courier New', monospace;
  color: #888;
}

.script-actions {
  display: flex;
  gap: 8px;
}

.btn-icon {
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.btn-icon:hover {
  background: #f0f0f0;
}

.btn-icon.active {
  background: #4caf50;
  color: white;
}

.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #666;
  padding: 40px;
}

.empty p {
  margin-bottom: 16px;
}

.footer {
  padding: 12px 16px;
  background: #f9f9f9;
  border-top: 1px solid #e0e0e0;
}
</style>
