<template>
  <div class="preview-panel panel-card">
    <div class="panel-header">
      <h3>预览结果</h3>
      <div class="actions">
        <label v-if="store.hasFiles" class="md5-mode-label" title="预览时如何计算 __utils.md5.file()">
          预览 MD5
          <select
            :value="store.previewMd5Mode"
            class="md5-mode-select"
            :disabled="store.previewInProgress"
            @change="store.setPreviewMd5Mode($event.target.value)"
          >
            <option value="auto">自动</option>
            <option value="always">始终计算</option>
            <option value="skip">始终跳过</option>
          </select>
        </label>
        <button
          v-if="store.previewInProgress"
          @click="store.cancelPreview"
          class="btn btn-secondary"
        >
          取消预览
        </button>
        <button
          v-else
          @click="store.previewRename"
            :disabled="!canPreview || store.loading || store.executeInProgress"
          class="btn btn-primary"
          title="预览 (Ctrl+Enter)"
        >
          预览
        </button>
      </div>
    </div>

    <div v-if="store.md5PreviewPlaceholder" class="md5-banner">
      <template v-if="store.previewMd5Mode === 'skip'">
        预览已跳过 MD5 计算（当前设置为「始终跳过」），新文件名中的 [md5-preview] 为占位符；执行时将计算真实 MD5。使用占位符时不检测路径冲突。
      </template>
      <template v-else-if="store.previewMd5Mode === 'auto'">
        预览已跳过部分 MD5 计算（文件较多或较大），新文件名中的 [md5-preview] 为占位符；执行时将计算真实 MD5。使用占位符时不检测路径冲突。
      </template>
      <template v-else>
        预览中部分文件使用了 MD5 占位符；执行时将计算真实 MD5。使用占位符时不检测路径冲突。
      </template>
    </div>

    <div v-if="store.previewStale && store.hasPreviewed" class="stale-banner">
      预览已过期（文件、脚本或参数已变更），请重新预览
    </div>

    <div v-if="store.hasConflicts" class="conflict-banner">
      <div class="conflict-banner-title">
        检测到 {{ store.conflictPaths.length }} 个目标路径冲突；继续执行时仅第一个会成功，其余可能失败
      </div>
      <ul class="conflict-list">
        <li v-for="item in store.conflictDetails" :key="item.newName" class="conflict-item">
          <span class="conflict-target">{{ item.newName }}</span>
          <span class="conflict-sources">← {{ item.originals.join('、') }}</span>
        </li>
      </ul>
    </div>

    <ScriptParamsForm
      v-if="store.currentScriptId && currentParameters.length > 0"
      :script-id="store.currentScriptId"
      :parameters="currentParameters"
      :expanded="expandedParams"
      @toggle="expandedParams = !expandedParams"
    />

    <div v-if="store.previewInProgress" class="preview-progress-section">
      <div class="preview-progress-bar">
        <div
          class="preview-progress-fill"
          :style="{ width: progressPercent + '%' }"
        ></div>
      </div>
      <div class="preview-progress-meta">
        <span class="preview-progress-text">{{ progressText }}</span>
        <span class="preview-progress-count">{{ progressPercent }}%</span>
      </div>
    </div>

    <div
      v-if="!store.previewInProgress && !store.hasPreviewed && !store.loading"
      class="empty-state"
    >
      <p>点击「预览」按钮查看重命名结果</p>
      <p class="hint">快捷键：Ctrl+Enter</p>
    </div>

    <div
      v-else-if="
        !store.previewInProgress &&
        store.hasPreviewed &&
        store.previewResults.length === 0 &&
        !store.loading
      "
      class="empty-state"
    >
      <div v-if="store.error" class="error-section">
        <div class="error-title">执行错误:</div>
        <div class="error-content">{{ store.error }}</div>
      </div>
      <p v-else class="no-changes">没有文件需要重命名，所有文件名保持不变</p>
      <SessionLogs :logs="displayLogs" :logs-dir="logsDir" />
    </div>

    <div v-if="store.previewResults.length > 0" class="preview-content">
      <div class="preview-toolbar">
        <input
          v-model="searchQuery"
          type="text"
          class="search-input"
          placeholder="搜索预览结果..."
        />
        <button
          v-if="useTreeView"
          @click="toggleAllTree"
          class="btn btn-secondary btn-sm"
        >
          {{ allExpanded ? '全部折叠' : '全部展开' }}
        </button>
      </div>

      <SessionLogs :logs="displayLogs" :logs-dir="logsDir" />

      <div v-if="useTreeView" class="preview-tree-wrap">
        <div
          v-for="(node, idx) in filteredTreeRoot.children"
          :key="'tree-' + idx + '-' + node.key"
          class="tree-node-wrap"
        >
          <PreviewTreeNode
            :node="node"
            :depth="0"
            :expanded-keys="expandedKeys"
            :get-path-parts="getPathParts"
            :has-changed="hasChanged"
            :is-conflict="isConflict"
            @toggle="toggleTreeExpand"
          />
        </div>
      </div>
      <template v-else>
        <div
          v-for="(result, index) in filteredResults"
          :key="index"
          class="preview-item"
          :class="{ error: result.error, conflict: isConflict(result) }"
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
          <div v-if="isConflict(result)" class="conflict-tag">路径冲突</div>
          <div v-if="result.error" class="error-message">{{ result.error }}</div>
        </div>
      </template>
    </div>

    <div
      v-if="store.previewResults.length > 0 && !store.previewInProgress"
      class="footer"
    >
      <div class="stats">
        <span>共 {{ store.previewResults.length }} 个文件</span>
        <span v-if="changedCount > 0">{{ changedCount }} 个文件将被重命名</span>
        <span v-if="store.hasConflicts" class="conflict-stat">存在路径冲突</span>
      </div>
      <button
        @click="requestExecute"
        :disabled="store.loading || store.previewStale || store.executeInProgress"
        class="btn btn-success"
        title="执行重命名 (Ctrl+Shift+Enter)"
      >
        {{ store.executeInProgress ? '执行中...' : '执行重命名' }}
      </button>
    </div>

    <ConfirmDialog
      :visible="showExecuteConfirm"
      title="确认执行重命名"
      :message="executeConfirmMessage"
      confirm-text="确认执行"
      confirm-variant="danger"
      @confirm="confirmExecute"
      @cancel="showExecuteConfirm = false"
    />
  </div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, ref, watch, defineComponent, h } from 'vue'
import { useRenameStore } from '../stores/renameStore'
import PreviewTreeNode from './PreviewTreeNode.vue'
import ScriptParamsForm from './ScriptParamsForm.vue'
import ConfirmDialog from './ui/ConfirmDialog.vue'

const SessionLogs = defineComponent({
  props: {
    logs: Array,
    logsDir: String,
  },
  setup(props) {
    return () =>
      h('div', { class: 'script-logs' }, [
        h('div', { class: 'logs-header', title: props.logsDir ? `日志目录: ${props.logsDir}` : '' }, [
          '脚本日志',
          props.logsDir ? h('span', { class: 'logs-persist-hint' }, '（本次预览）') : null,
        ]),
        h('div', { class: 'logs-content' }, [
          props.logs?.length
            ? props.logs.map((log, i) => h('div', { class: 'log-item', key: i }, log))
            : h('div', { class: 'log-empty' }, '暂无日志输出'),
        ]),
      ])
  },
})

const store = useRenameStore()
const expandedParams = ref(true)
const logsDir = ref('')
const searchQuery = ref('')
const showExecuteConfirm = ref(false)
const expandedKeys = ref(new Set())
const allExpanded = ref(false)

const TREE_VIEW_THRESHOLD = 10

onMounted(async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    logsDir.value = await invoke('get_logs_dir_display')
  } catch {
    // 非 Tauri 环境
  }
  window.addEventListener('request-execute-rename', requestExecute)
})

onUnmounted(() => {
  window.removeEventListener('request-execute-rename', requestExecute)
})

watch(
  () => store.currentScriptId,
  (id) => {
    if (id && store.getCurrentScriptParameters().length > 0) {
      expandedParams.value = true
    }
  }
)

const currentParameters = computed(() => store.getCurrentScriptParameters())

const useTreeView = computed(() => store.previewResults.length >= TREE_VIEW_THRESHOLD)

const progressPercent = computed(() => {
  const { completed, total } = store.previewProgress
  if (!total) return 0
  return Math.min(100, Math.round((completed / total) * 100))
})

const progressText = computed(() => {
  const { completed, total, currentPath, phase } = store.previewProgress
  const base = total > 0 ? `已处理 ${completed} / ${total} 个文件` : '准备预览...'
  if (!currentPath) return base
  const name = currentPath.split(/[/\\]/).pop() || currentPath
  if (phase === 'md5') return `${base} · 计算 MD5: ${name}`
  if (phase === 'script') return `${base} · 执行脚本: ${name}`
  return base
})

const displayLogs = computed(() =>
  store.sessionScriptLogs.length > 0 ? store.sessionScriptLogs : store.scriptLogs
)

const filteredResults = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return store.previewResults
  return store.previewResults.filter(
    (r) =>
      r.original.toLowerCase().includes(q) ||
      r.new_name.toLowerCase().includes(q)
  )
})

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

function filterTree(node, query) {
  if (node.type === 'file') {
    const r = node.result
    const match =
      !query ||
      r.original.toLowerCase().includes(query) ||
      r.new_name.toLowerCase().includes(query)
    return match ? { ...node } : null
  }
  const filteredChildren = (node.children || [])
    .map((c) => filterTree(c, query))
    .filter(Boolean)
  if (filteredChildren.length === 0) return null
  return { ...node, children: filteredChildren }
}

const filteredTreeRoot = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return previewTreeRoot.value
  const filtered = (previewTreeRoot.value.children || [])
    .map((c) => filterTree(c, q))
    .filter(Boolean)
  return { ...previewTreeRoot.value, children: filtered }
})

watch(
  () => store.previewResults.length,
  () => {
    if (useTreeView.value) {
      const keys = new Set()
      for (const node of previewTreeRoot.value.children) {
        if (node.type === 'dir') keys.add(node.key)
      }
      expandedKeys.value = keys
      allExpanded.value = true
    }
  },
  { immediate: true }
)

function toggleTreeExpand(key) {
  const next = new Set(expandedKeys.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expandedKeys.value = next
}

function collectDirKeys(nodes, keys = []) {
  for (const n of nodes || []) {
    if (n.type === 'dir') {
      keys.push(n.key)
      collectDirKeys(n.children, keys)
    }
  }
  return keys
}

function toggleAllTree() {
  if (allExpanded.value) {
    expandedKeys.value = new Set()
    allExpanded.value = false
  } else {
    expandedKeys.value = new Set(collectDirKeys(previewTreeRoot.value.children))
    allExpanded.value = true
  }
}

const canPreview = computed(() => store.hasFiles && store.scriptValid)

const changedCount = computed(() => store.changedPreviewCount)

const executeConfirmMessage = computed(() => {
  const count = changedCount.value
  let conflict = ''
  if (store.hasConflicts) {
    const lines = store.conflictDetails.map(
      (item) => `• ${item.newName}\n  ← ${item.originals.join('、')}`
    )
    conflict = `\n\n警告：以下 ${store.conflictPaths.length} 个目标路径存在冲突（多个源文件将重命名为同一路径，仅第一个会成功）：\n${lines.join('\n')}`
  }
  return `即将重命名 ${count} 个文件。此操作将直接修改磁盘文件，关闭应用后无法撤销（仅支持应用内撤销）。${conflict}\n\n确定继续？`
})

const hasChanged = (result) => result.original !== result.new_name && !result.error

const MD5_PREVIEW_PLACEHOLDER = '[md5-preview]'

const isConflict = (result) =>
  !result.new_name.includes(MD5_PREVIEW_PLACEHOLDER) &&
  store.conflictPaths.includes(result.new_name)

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

function requestExecute() {
  if (store.previewStale) {
    store.previewRename().then(() => {
      if (store.hasPreview) showExecuteConfirm.value = true
    })
    return
  }
  if (!store.hasPreview) return
  showExecuteConfirm.value = true
}

async function confirmExecute() {
  showExecuteConfirm.value = false
  await store.executeRename(true)
}
</script>

<style scoped>
.preview-panel {
  position: relative;
}

.actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.stale-banner,
.conflict-banner,
.md5-banner,
.preview-progress-section {
  padding: 8px 16px;
  font-size: 13px;
  flex-shrink: 0;
}

.preview-progress-section {
  background: #f0f7ff;
  border-bottom: 1px solid #cfe2ff;
}

.preview-progress-bar {
  height: 6px;
  background: #dde8f7;
  border-radius: 3px;
  overflow: hidden;
}

.preview-progress-fill {
  height: 100%;
  background: #007bff;
  border-radius: 3px;
  transition: width 0.2s ease;
}

.preview-progress-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 6px;
  font-size: 12px;
  color: #555;
}

.preview-progress-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-progress-count {
  flex-shrink: 0;
  font-weight: 600;
  color: #007bff;
}

.md5-banner {
  background: #e7f3ff;
  color: #0c5460;
  border-bottom: 1px solid #b8daff;
}

.stale-banner {
  background: #fff3cd;
  color: #856404;
  border-bottom: 1px solid #ffc107;
}

.conflict-banner {
  background: #f8d7da;
  color: #721c24;
  border-bottom: 1px solid #f5c6cb;
}

.conflict-banner-title {
  margin-bottom: 6px;
}

.conflict-list {
  margin: 0;
  padding: 0 0 0 18px;
  font-size: 12px;
  line-height: 1.5;
}

.conflict-item {
  margin-top: 4px;
  word-break: break-all;
}

.conflict-target {
  font-weight: 600;
  font-family: Monaco, 'Courier New', monospace;
}

.conflict-sources {
  display: block;
  margin-top: 2px;
  color: #a94442;
  font-family: Monaco, 'Courier New', monospace;
}

.hint {
  font-size: 12px;
  color: #999;
  margin-top: 8px;
}

.no-changes {
  color: #28a745;
  font-weight: 500;
}

.preview-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px;
}

.preview-toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.search-input {
  flex: 1;
  min-width: 120px;
  padding: 6px 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 13px;
}

.md5-mode-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #666;
  white-space: nowrap;
  flex-shrink: 0;
}

.md5-mode-select {
  padding: 5px 8px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 12px;
  background: #fff;
  max-width: 180px;
}

.btn-sm {
  padding: 4px 10px;
  font-size: 12px;
  white-space: nowrap;
}

.preview-tree-wrap {
  margin-top: 8px;
  padding: 8px;
  background: #fafafa;
  border: 1px solid #e8e8e8;
  border-radius: 6px;
}

.preview-item {
  padding: 12px;
  margin-bottom: 8px;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
}

.preview-item.conflict {
  border-color: #dc3545;
  background: #fff5f5;
}

.preview-item:hover {
  border-color: #007bff;
}

.preview-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.original, .new {
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

.value.changed { color: #28a745; font-weight: 600; }

.path-diff-removed { background: rgba(220, 53, 69, 0.15); color: #c82333; font-weight: 600; }
.path-diff-added { background: rgba(40, 167, 69, 0.2); color: #1e7e34; font-weight: 600; }
.path-common { color: #666; }

.arrow { font-size: 20px; color: #999; }

.conflict-tag {
  margin-top: 6px;
  font-size: 12px;
  color: #dc3545;
  font-weight: 600;
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

.conflict-stat { color: #dc3545; font-weight: 600; }

:deep(.script-logs) {
  margin-bottom: 12px;
  padding: 12px;
  background: #f8f9fa;
  border: 1px solid #dee2e6;
  border-radius: 4px;
}

:deep(.logs-header) { font-size: 12px; font-weight: 600; color: #495057; margin-bottom: 8px; }
:deep(.logs-persist-hint) { margin-left: 4px; font-weight: 400; color: #868e96; }
:deep(.logs-content) { font-family: Monaco, monospace; font-size: 12px; background: #fff; padding: 8px; border-radius: 2px; }
:deep(.log-item) { padding: 2px 0; white-space: pre-wrap; word-break: break-all; }
:deep(.log-empty) { color: #999; font-style: italic; text-align: center; padding: 8px 0; }

.error-section {
  margin-bottom: 16px;
  padding: 12px;
  background: #fee;
  border: 1px solid #fcc;
  border-radius: 4px;
  width: 100%;
}

.error-title { font-weight: 600; color: #c33; margin-bottom: 8px; }
.error-content { color: #c33; font-size: 13px; white-space: pre-wrap; }
</style>
