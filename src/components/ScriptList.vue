<template>
  <div class="script-list panel-card">
    <div class="panel-header">
      <h3>脚本列表</h3>
      <div class="actions">
        <button @click="openEditor" class="btn btn-primary">新建脚本</button>
        <button @click="reloadScripts" class="btn btn-secondary" title="重新加载 (Ctrl+R)">刷新</button>
        <button @click="openScriptsDir" class="btn btn-secondary">脚本目录</button>
      </div>
    </div>

    <div v-if="store.currentScriptId" class="current-script-banner">
      <span class="current-script-label">当前使用:</span>
      <span class="current-script-name">{{ currentScriptName }}</span>
    </div>

    <div class="quick-rules">
      <div class="quick-rules-title">快速规则</div>
      <div class="quick-rules-list">
        <button
          v-for="rule in quickRules"
          :key="rule.id"
          class="quick-rule-btn"
          :class="{ active: store.currentScriptId === rule.id }"
          :title="rule.description"
          @click="selectQuickRule(rule.id)"
        >
          {{ rule.name }}
        </button>
      </div>
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
          <div class="script-name">{{ script.display_name || script.name }}</div>
          <div v-if="script.description" class="script-description">{{ script.description }}</div>
          <div class="script-meta">
            <span class="script-file-name">{{ script.name }}</span>
            <span>{{ formatDate(script.updated_at) }}</span>
          </div>
        </div>
        <div class="script-actions" @click.stop>
          <button @click="openRenameDialog(script.id, script.name)" class="btn-icon" title="重命名">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          </button>
          <button @click="editScript(script.id)" class="btn-icon" title="编辑">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
          </button>
          <button @click="editManifest(script.id)" class="btn-icon" title="编辑信息">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
          </button>
          <button @click="openDeleteDialog(script.id)" class="btn-icon danger" title="删除">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
          </button>
        </div>
      </div>

      <div v-if="store.savedScripts.length === 0" class="empty-state">
        <p>暂无保存的脚本</p>
        <button @click="openEditor" class="btn btn-primary">创建第一个脚本</button>
      </div>
    </div>

    <ConfirmDialog
      :visible="showDeleteConfirm"
      title="删除脚本"
      message="确定要删除这个脚本吗？此操作不可恢复。"
      confirm-text="删除"
      confirm-variant="danger"
      @confirm="confirmDelete"
      @cancel="showDeleteConfirm = false"
    />

    <PromptDialog
      :visible="showRenameDialog"
      title="重命名脚本"
      message="请输入新的脚本名称："
      :default-value="renameDefaultValue"
      @confirm="confirmRename"
      @cancel="showRenameDialog = false"
    />
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRenameStore } from '../stores/renameStore'
import { QUICK_RULES } from '../constants/quickRules'
import { openScriptEditor } from '../composables/scriptEditor'
import ConfirmDialog from './ui/ConfirmDialog.vue'
import PromptDialog from './ui/PromptDialog.vue'

async function getInvoke() {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke
}

const store = useRenameStore()
const quickRules = QUICK_RULES
const showDeleteConfirm = ref(false)
const showRenameDialog = ref(false)
const pendingDeleteId = ref(null)
const pendingRenameId = ref(null)
const renameDefaultValue = ref('')
let manifestPollTimer = null

const currentScriptName = computed(() => {
  if (!store.currentScriptId) return ''
  const saved = store.savedScripts.find((s) => s.id === store.currentScriptId)
  if (saved) return saved.display_name || saved.name
  const quick = quickRules.find((r) => r.id === store.currentScriptId)
  return quick ? quick.name : store.currentScriptId
})

onMounted(async () => {
  await store.initSavedScripts()
})

onUnmounted(() => {
  if (manifestPollTimer) clearInterval(manifestPollTimer)
})

const formatDate = (dateString) => {
  return new Date(dateString).toLocaleString('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit',
  })
}

const selectScript = (scriptId) => store.loadScript(scriptId)
const selectQuickRule = (ruleId) => store.loadQuickRule(ruleId)

const editScript = async (scriptId) => {
  try {
    await openScriptEditor(scriptId)
  } catch (error) {
    store.error = `打开脚本编辑器失败: ${error}`
  }
}

const openEditor = async () => {
  try {
    const invoke = await getInvoke()
    const tempName = `new_script_${Date.now()}`
    const tempScript = `function rename() {
  return {[__filePath]: __fileName};
}`
    await invoke('save_script', { name: tempName, script: tempScript, scriptId: null })
    await store.initSavedScripts()
    await openScriptEditor(tempName)
  } catch (error) {
    store.error = `创建新脚本失败: ${error}`
  }
}

const openDeleteDialog = (scriptId) => {
  pendingDeleteId.value = scriptId
  showDeleteConfirm.value = true
}

const confirmDelete = async () => {
  showDeleteConfirm.value = false
  if (pendingDeleteId.value) {
    await store.deleteScript(pendingDeleteId.value)
    pendingDeleteId.value = null
  }
}

const openRenameDialog = (scriptId, currentName) => {
  pendingRenameId.value = scriptId
  renameDefaultValue.value = currentName
  showRenameDialog.value = true
}

const confirmRename = async (newName) => {
  showRenameDialog.value = false
  const scriptId = pendingRenameId.value
  if (!scriptId || !newName) return

  const success = await store.renameScript(scriptId, newName)
  if (success) {
    await store.initSavedScripts()
    if (store.currentScriptId === scriptId) {
      const safeName = newName.trim().replace(/[<>:"/\\|?*]/g, '_')
      const newScript = store.savedScripts.find(
        (s) => s.id === safeName || s.name === newName.trim()
      )
      if (newScript) await store.loadScript(newScript.id)
    }
  }
  pendingRenameId.value = null
}

const reloadScripts = () => store.initSavedScripts()

const openScriptsDir = async () => {
  try {
    const invoke = await getInvoke()
    await invoke('open_script_file', { scriptId: '.' })
  } catch (error) {
    store.error = `打开脚本目录失败: ${error}`
  }
}

const editManifest = async (scriptId) => {
  try {
    const invoke = await getInvoke()
    await invoke('open_script_manifest', { scriptId })
    startManifestPolling()
  } catch (error) {
    store.error = `打开脚本信息文件失败: ${error}`
  }
}

function startManifestPolling() {
  if (manifestPollTimer) clearInterval(manifestPollTimer)
  let count = 0
  manifestPollTimer = setInterval(async () => {
    await store.initSavedScripts()
    count++
    if (count >= 30) {
      clearInterval(manifestPollTimer)
      manifestPollTimer = null
    }
  }, 2000)
}
</script>

<style scoped>
.current-script-banner {
  padding: 8px 16px;
  background: #e3f2fd;
  border-bottom: 1px solid #90caf9;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.current-script-label { color: #1976d2; font-weight: 500; }
.current-script-name { color: #1565c0; font-weight: 600; }

.quick-rules {
  padding: 10px 12px;
  border-bottom: 1px solid #e0e0e0;
  background: #fafafa;
}

.quick-rules-title {
  font-size: 12px;
  color: #666;
  font-weight: 600;
  margin-bottom: 8px;
}

.quick-rules-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.quick-rule-btn {
  padding: 4px 10px;
  border: 1px solid #ddd;
  border-radius: 12px;
  background: white;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.quick-rule-btn:hover {
  border-color: #007bff;
  color: #007bff;
}

.quick-rule-btn.active {
  background: #007bff;
  color: white;
  border-color: #007bff;
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

.script-item:hover { border-color: #007bff; background: #f8f9fa; }
.script-item.active { border-color: #007bff; background: #e7f3ff; }

.script-info { flex: 1; min-width: 0; }
.script-name { font-weight: 500; color: #333; margin-bottom: 4px; }
.script-description { font-size: 13px; color: #666; margin-bottom: 4px; line-height: 1.4; }
.script-meta { font-size: 12px; color: #666; display: flex; gap: 12px; flex-wrap: wrap; }
.script-file-name { font-family: Monaco, monospace; color: #888; }
.script-actions { display: flex; gap: 4px; flex-shrink: 0; }
</style>
