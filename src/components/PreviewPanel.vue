<template>
  <div class="preview-panel">
    <div class="header">
      <h3>预览结果</h3>
      <div class="actions">
        <button
          @click="store.previewRename"
          :disabled="!canPreview || store.loading"
          class="btn btn-primary"
        >
          {{ store.loading ? '预览中...' : '预览' }}
        </button>
      </div>
    </div>

    <!-- 参数输入区域 -->
    <div v-if="currentScript && currentScript.parameters && currentScript.parameters.length > 0" class="script-params-preview">
      <div class="params-header" @click="toggleParamsPreview">
        <span>脚本参数</span>
        <span class="params-toggle">{{ expandedParamsPreview ? '▼' : '▶' }}</span>
      </div>
      <div v-if="expandedParamsPreview" class="params-content">
        <div
          v-for="param in currentScript.parameters"
          :key="param.name"
          class="param-item"
        >
          <label class="param-label">
            {{ param.name }}
            <span v-if="param.required" class="required">*</span>
            <span v-if="param.description" class="param-desc">({{ param.description }})</span>
          </label>
          <input
            v-if="param.type === 'string'"
            type="text"
            :value="getParamValue(param.name)"
            @input="updateParam(param.name, $event.target.value)"
            class="param-input"
            :class="{ error: isParamInvalid(param) }"
          />
          <input
            v-else-if="param.type === 'number'"
            type="number"
            :value="getParamValue(param.name)"
            @input="updateParam(param.name, parseFloat($event.target.value) || 0)"
            class="param-input"
            :class="{ error: isParamInvalid(param) }"
          />
          <label v-else-if="param.type === 'boolean'" class="param-checkbox">
            <input
              type="checkbox"
              :checked="getParamValue(param.name)"
              @change="updateParam(param.name, $event.target.checked)"
            />
            <span>{{ param.description || param.name }}</span>
          </label>
        </div>
      </div>
    </div>

    <div v-if="store.loading" class="loading">
      <p>正在预览...</p>
    </div>

    <div v-else-if="!store.hasPreviewed" class="empty">
      <p>点击"预览"按钮查看重命名结果</p>
    </div>

    <div v-else-if="store.hasPreviewed && store.previewResults.length === 0" class="empty">
      <!-- 显示错误信息 -->
      <div v-if="store.error" class="error-section">
        <div class="error-title">执行错误:</div>
        <div class="error-content">{{ store.error }}</div>
      </div>
      <p v-else class="no-changes">✓ 没有文件需要重命名，所有文件名保持不变</p>
      <!-- 即使没有预览结果，也显示日志 -->
      <div class="script-logs" style="margin-top: 16px;">
        <div class="logs-header" :title="logsDir ? `日志目录: ${logsDir}` : ''">
          脚本日志<span v-if="logsDir" class="logs-persist-hint">（已持久化）</span>
        </div>
        <div class="logs-content">
          <div v-if="store.scriptLogs && store.scriptLogs.length > 0">
            <div v-for="(log, index) in store.scriptLogs" :key="index" class="log-item">
              {{ log }}
            </div>
          </div>
          <div v-else class="log-empty">暂无日志输出</div>
        </div>
      </div>
    </div>

    <div v-else class="preview-content">
      <!-- 固定显示脚本日志区域 -->
      <div class="script-logs">
        <div class="logs-header" :title="logsDir ? `日志目录: ${logsDir}` : ''">
          脚本日志<span v-if="logsDir" class="logs-persist-hint">（已持久化）</span>
        </div>
        <div class="logs-content">
          <div v-if="store.scriptLogs && store.scriptLogs.length > 0">
            <div v-for="(log, index) in store.scriptLogs" :key="index" class="log-item">
              {{ log }}
            </div>
          </div>
          <div v-else class="log-empty">暂无日志输出</div>
        </div>
      </div>

      <!-- 预览结果：数量多时用文件夹树，否则用平铺列表 -->
      <div v-if="useTreeView" class="preview-tree-wrap">
        <div
          v-for="(node, idx) in previewTreeRoot.children"
          :key="'tree-' + idx + '-' + node.key"
          class="tree-node-wrap"
        >
          <PreviewTreeNode
            :node="node"
            :depth="0"
            :expanded-keys="expandedKeys"
            :get-path-parts="getPathParts"
            :has-changed="hasChanged"
            @toggle="toggleTreeExpand(node.key)"
          />
        </div>
      </div>
      <template v-else>
        <div
          v-for="(result, index) in store.previewResults"
          :key="index"
          class="preview-item"
          :class="{ error: result.error }"
        >
          <div class="preview-info">
            <div class="original">
              <span class="label">原路径:</span>
              <span class="value path-value">
                <template v-for="(part, i) in getPathParts(result.original, result.new_name).original" :key="'o-' + i">
                  <span :class="part.type === 'common' ? 'path-common' : 'path-diff path-diff-removed'">{{ part.text }}</span>
                </template>
              </span>
            </div>
            <div class="arrow">→</div>
            <div class="new">
              <span class="label">新路径:</span>
              <span class="value path-value" :class="{ changed: hasChanged(result) }">
                <template v-for="(part, i) in getPathParts(result.original, result.new_name).new" :key="'n-' + i">
                  <span :class="part.type === 'common' ? 'path-common' : 'path-diff path-diff-added'">{{ part.text }}</span>
                </template>
              </span>
            </div>
          </div>
          <div v-if="result.error" class="error-message">
            {{ result.error }}
          </div>
        </div>
      </template>
    </div>

    <div v-if="store.hasPreviewed && store.previewResults.length > 0" class="footer">
      <div class="stats">
        <span>共 {{ store.previewResults.length }} 个文件</span>
        <span v-if="changedCount > 0">
          {{ changedCount }} 个文件将被重命名
        </span>
      </div>
      <button
        @click="store.executeRename"
        :disabled="store.loading"
        class="btn btn-success"
      >
        {{ store.loading ? '执行中...' : '执行重命名' }}
      </button>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { useRenameStore } from '../stores/renameStore'
import PreviewTreeNode from './PreviewTreeNode.vue'

const store = useRenameStore()
const expandedParamsPreview = ref(false)
const logsDir = ref('')

onMounted(async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    logsDir.value = await invoke('get_logs_dir_display')
  } catch {
    // 非 Tauri 环境忽略
  }
})

/** 超过该数量时预览以文件夹树展示 */
const TREE_VIEW_THRESHOLD = 10

const useTreeView = computed(() => store.previewResults.length >= TREE_VIEW_THRESHOLD)

const expandedKeys = ref(new Set())

function toggleTreeExpand(key) {
  const next = new Set(expandedKeys.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expandedKeys.value = next
}

/** 树视图下默认展开第一层目录 */
watch(
  () => store.previewResults.length,
  () => {
    if (useTreeView.value) {
      const keys = new Set()
      for (const node of previewTreeRoot.value.children) {
        if (node.type === 'dir') keys.add(node.key)
      }
      expandedKeys.value = keys
    }
  },
  { immediate: true }
)

/** 将预览结果按路径构建为树 { name, type: 'dir'|'file', key, children?, result? } */
const previewTreeRoot = computed(() => {
  const sep = /[/\\]/
  const root = { name: '', type: 'dir', key: '__root__', children: [] }
  for (const r of store.previewResults) {
    const segments = r.original.split(sep).filter(Boolean)
    if (segments.length === 0) {
      root.children.push({ type: 'file', name: r.original, key: r.original, result: r })
      continue
    }
    const fileName = segments.pop()
    const sepChar = r.original.includes('\\') ? '\\' : '/'
    let current = root
    let pathSoFar = ''
    for (const seg of segments) {
      const nextPath = pathSoFar ? pathSoFar + sepChar + seg : seg
      let child = current.children.find((c) => c.name === seg && c.type === 'dir')
      if (!child) {
        child = { type: 'dir', name: seg, key: nextPath, children: [] }
        current.children.push(child)
      }
      current = child
      pathSoFar = nextPath
    }
    current.children.push({ type: 'file', name: fileName, key: r.original, result: r })
  }
  // 目录按名称排序，文件保持顺序
  function sortChildren(n) {
    if (n.children) {
      n.children.sort((a, b) => {
        if (a.type !== b.type) return a.type === 'dir' ? -1 : 1
        return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })
      })
      n.children.forEach(sortChildren)
    }
  }
  sortChildren(root)
  return root
})

// 获取当前脚本
const currentScript = computed(() => {
  if (!store.currentScriptId) {
    return null
  }
  return store.savedScripts.find((s) => s.id === store.currentScriptId)
})

// 切换参数区域的展开/折叠
const toggleParamsPreview = () => {
  expandedParamsPreview.value = !expandedParamsPreview.value
}

// 获取参数值
const getParamValue = (paramName) => {
  if (!store.currentScriptId) {
    return undefined
  }
  const params = store.scriptParams[store.currentScriptId]
  if (!params) {
    return undefined
  }
  return params[paramName]
}

// 更新参数值
const updateParam = (paramName, value) => {
  if (!store.currentScriptId) {
    return
  }
  store.setScriptParam(store.currentScriptId, paramName, value)
}

// 检查参数是否无效
const isParamInvalid = (param) => {
  if (!param.required) {
    return false
  }
  const value = getParamValue(param.name)
  if (value === undefined || value === null || value === '') {
    return true
  }
  if (param.type === 'number' && isNaN(value)) {
    return true
  }
  return false
}

const canPreview = computed(() => {
  return store.hasFiles && store.scriptValid
})


const changedCount = computed(() => {
  return store.previewResults.filter((r) => hasChanged(r)).length
})

const hasChanged = (result) => {
  return result.original !== result.new_name && !result.error
}

/** 路径按分隔符拆成段，用于高亮展示差异 */
const pathSeparator = /[/\\]/
function getPathParts(original, newName) {
  const a = original.split(pathSeparator).filter(Boolean)
  const b = newName.split(pathSeparator).filter(Boolean)
  let i = 0
  while (i < a.length && i < b.length && a[i] === b[i]) i++
  const sep = original.includes('\\') ? '\\' : '/'
  const commonPath = a.slice(0, i).join(sep)
  const originalTail = a.slice(i).join(sep)
  const newTail = b.slice(i).join(sep)

  const originalParts = []
  if (commonPath) originalParts.push({ type: 'common', text: commonPath + (originalTail ? sep : '') })
  if (originalTail) originalParts.push({ type: 'diff', text: originalTail })

  const newParts = []
  if (commonPath) newParts.push({ type: 'common', text: commonPath + (newTail ? sep : '') })
  if (newTail) newParts.push({ type: 'diff', text: newTail })

  return {
    original: originalParts.length ? originalParts : [{ type: 'common', text: original }],
    new: newParts.length ? newParts : [{ type: 'common', text: newName }],
  }
}
</script>

<style scoped>
.preview-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  max-height: 100%;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow: hidden;
  position: relative;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: #f5f5f5;
  border-bottom: 1px solid #e0e0e0;
  flex-shrink: 0;
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

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-primary {
  background: #007bff;
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: #0056b3;
}

.btn-success {
  background: #28a745;
  color: white;
}

.btn-success:hover:not(:disabled) {
  background: #218838;
}

.loading,
.empty {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  color: #666;
  padding: 16px;
}

.empty .no-changes {
  color: #28a745;
  font-weight: 500;
  font-size: 14px;
}

.preview-content {
  flex: 1;
  min-height: 0;
  padding: 8px;
}

.preview-tree-wrap {
  margin-top: 8px;
  padding: 8px;
  background: #fafafa;
  border: 1px solid #e8e8e8;
  border-radius: 6px;
  max-height: 360px;
  overflow-y: auto;
}

.preview-item {
  padding: 12px;
  margin-bottom: 8px;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  transition: border-color 0.2s;
}

.preview-item:hover {
  border-color: #007bff;
}

.preview-item.error {
  border-color: #dc3545;
  background: #fff5f5;
}

.preview-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.original,
.new {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.label {
  font-size: 12px;
  color: #666;
  font-weight: 500;
}

.value {
  font-family: Monaco, 'Courier New', monospace;
  font-size: 14px;
  color: #333;
  word-break: break-all;
}

.value.changed {
  color: #28a745;
  font-weight: 600;
}

.path-value {
  display: inline;
}

.path-common {
  color: #666;
}

.path-diff {
  font-weight: 600;
  padding: 0 1px;
  border-radius: 2px;
}

.path-diff-removed {
  background: rgba(220, 53, 69, 0.15);
  color: #c82333;
}

.path-diff-added {
  background: rgba(40, 167, 69, 0.2);
  color: #1e7e34;
}

.arrow {
  font-size: 20px;
  color: #999;
  padding: 0 8px;
}

.error-message {
  margin-top: 8px;
  padding: 8px;
  background: #ffe6e6;
  border-left: 3px solid #dc3545;
  color: #dc3545;
  font-size: 12px;
}

.footer {
  padding: 12px 16px;
  background: #f9f9f9;
  border-top: 1px solid #e0e0e0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}

.stats {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: #666;
}

.script-logs {
  margin-bottom: 16px;
  padding: 12px;
  background: #f8f9fa;
  border: 1px solid #dee2e6;
  border-radius: 4px;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
}

.logs-header {
  font-size: 12px;
  font-weight: 600;
  color: #495057;
  margin-bottom: 8px;
}

.logs-persist-hint {
  margin-left: 4px;
  font-weight: 400;
  color: #868e96;
}

.logs-content {
  font-family: Monaco, 'Courier New', monospace;
  font-size: 12px;
  background: #fff;
  padding: 8px;
  border-radius: 2px;
}

.log-item {
  padding: 2px 0;
  color: #333;
  white-space: pre-wrap;
  word-break: break-all;
}

.log-empty {
  padding: 8px 0;
  color: #999;
  font-style: italic;
  text-align: center;
}

.error-section {
  margin-bottom: 16px;
  padding: 12px;
  background: #fee;
  border: 1px solid #fcc;
  border-radius: 4px;
}

.error-title {
  font-weight: 600;
  color: #c33;
  margin-bottom: 8px;
  font-size: 14px;
}

.error-content {
  color: #c33;
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-all;
}

.script-params-preview {
  padding: 12px 16px;
  background: #f9f9f9;
  border-bottom: 1px solid #e0e0e0;
}

.params-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  cursor: pointer;
  font-size: 13px;
  color: #666;
  font-weight: 500;
}

.params-header:hover {
  color: #007bff;
}

.params-toggle {
  font-size: 12px;
  color: #999;
}

.params-content {
  padding: 8px 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.param-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.param-label {
  font-size: 13px;
  color: #333;
  font-weight: 500;
}

.param-label .required {
  color: #dc3545;
  margin-left: 2px;
}

.param-label .param-desc {
  color: #666;
  font-weight: normal;
  font-size: 12px;
  margin-left: 4px;
}

.param-input {
  padding: 6px 8px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 13px;
  width: 100%;
}

.param-input:focus {
  outline: none;
  border-color: #007bff;
}

.param-input.error {
  border-color: #dc3545;
}

.param-checkbox {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  cursor: pointer;
}

.param-checkbox input[type="checkbox"] {
  cursor: pointer;
}
</style>
