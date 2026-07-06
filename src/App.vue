<template>
  <div class="app">
    <div class="app-header">
      <div class="header-top">
        <h1>Power Renamer</h1>
        <div class="header-actions">
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
          <button
            @click="requestExecute"
            :disabled="!canExecute || store.loading || store.previewInProgress || store.executeInProgress"
            class="btn btn-success"
            title="执行重命名 (Ctrl+Shift+Enter)"
          >
            {{ store.executeInProgress ? '执行中...' : '执行重命名' }}
          </button>
          <span class="header-divider" aria-hidden="true"></span>
          <button
            v-if="store.canUndo"
            @click="store.undo"
            :disabled="store.loading"
            class="btn btn-warning"
            title="撤销 (Ctrl+Z)"
          >
            撤销
          </button>
          <button
            @click="store.toggleHistoryDrawer"
            class="btn btn-secondary"
            title="历史记录"
          >
            历史记录
          </button>
        </div>
      </div>
      <StepIndicator />

      <div v-if="store.executeInProgress" class="execute-progress-section">
        <div class="execute-progress-bar">
          <div
            class="execute-progress-fill"
            :style="{ width: executeProgressPercent + '%' }"
          ></div>
        </div>
        <div class="execute-progress-meta">
          <span class="execute-progress-text">{{ executeProgressText }}</span>
          <span class="execute-progress-count">{{ executeProgressPercent }}%</span>
        </div>
      </div>

      <div v-if="store.error" class="error-banner">
        <span>{{ store.error }}</span>
        <button @click="store.clearError" class="btn-close-error">×</button>
      </div>
    </div>

    <div class="app-content" :class="{ narrow: isNarrow }">
      <nav v-if="isNarrow" class="panel-tabs" aria-label="工作区切换">
        <button
          v-for="tab in panelTabs"
          :key="tab.id"
          type="button"
          class="panel-tab"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          {{ tab.label }}
          <span v-if="tab.badge" class="panel-tab-badge">{{ tab.badge }}</span>
        </button>
      </nav>

      <div
        v-show="!isNarrow || activeTab === 'files'"
        class="left-panel resizable-panel"
        :style="isNarrow ? undefined : { width: leftWidth + 'px' }"
      >
        <FileSelector />
        <div v-if="!isNarrow" class="resize-handle" @mousedown="startResize('left', $event)"></div>
      </div>

      <div
        v-show="!isNarrow || activeTab === 'scripts'"
        class="middle-panel resizable-panel"
        :style="isNarrow ? undefined : { width: middleWidth + 'px' }"
      >
        <ScriptList />
        <div v-if="!isNarrow" class="resize-handle" @mousedown="startResize('middle', $event)"></div>
      </div>

      <div
        v-show="!isNarrow || activeTab === 'preview'"
        class="right-panel"
      >
        <PreviewPanel />
      </div>
    </div>

    <HistoryPanel :drawer="true" />
  </div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRenameStore } from './stores/renameStore'
import FileSelector from './components/FileSelector.vue'
import ScriptList from './components/ScriptList.vue'
import PreviewPanel from './components/PreviewPanel.vue'
import HistoryPanel from './components/HistoryPanel.vue'
import StepIndicator from './components/StepIndicator.vue'
import { onScriptSaved, onScriptsChanged } from './composables/scriptEditor'

const store = useRenameStore()
const canPreview = computed(() => store.hasFiles && store.scriptValid)
const canExecute = computed(
  () =>
    store.hasPreviewed &&
    store.hasPreview &&
    !store.previewStale &&
    !store.executeInProgress
)

const executeProgressPercent = computed(() => {
  const { completed, total } = store.executeProgress
  if (!total) return 0
  return Math.min(100, Math.round((completed / total) * 100))
})

const executeProgressText = computed(() => {
  const { completed, total, currentPath, phase } = store.executeProgress
  const base = total > 0 ? `已处理 ${completed} / ${total}` : '准备执行...'
  if (!currentPath) return base
  const name = currentPath.split(/[/\\]/).pop() || currentPath
  if (phase === 'md5') return `${base} · 计算 MD5: ${name}`
  if (phase === 'script') return `${base} · 执行脚本: ${name}`
  if (phase === 'rename') return `${base} · 重命名: ${name}`
  return base
})
const leftWidth = ref(300)
const middleWidth = ref(400)
const isNarrow = ref(false)
const activeTab = ref('files')
const NARROW_BREAKPOINT = 1100

const panelTabs = computed(() => [
  { id: 'files', label: '选择文件', badge: store.files.length || null },
  { id: 'scripts', label: '脚本', badge: null },
  {
    id: 'preview',
    label: '预览',
    badge: store.previewResults.length || null,
  },
])

function updateLayoutMode() {
  isNarrow.value = window.innerWidth < NARROW_BREAKPOINT
}

watch(
  () => store.workflowStep,
  (step) => {
    if (isNarrow.value && step >= 3) {
      activeTab.value = 'preview'
    }
  }
)

watch(
  () => store.previewInProgress,
  (inProgress) => {
    if (inProgress && isNarrow.value) {
      activeTab.value = 'preview'
    }
  }
)

watch(
  () => store.executeInProgress,
  (inProgress) => {
    if (inProgress && isNarrow.value) {
      activeTab.value = 'preview'
    }
  }
)

watch(isNarrow, (narrow) => {
  if (!narrow) return
  if (store.workflowStep >= 3 || store.previewInProgress) {
    activeTab.value = 'preview'
  }
})
let resizeTarget = null
let startX = 0
let startWidth = 0
let unlisteners = []

onMounted(async () => {
  updateLayoutMode()
  window.addEventListener('resize', updateLayoutMode)
  await store.initSavedScripts()
  await store.initScriptLogs()
  await store.initHistory()

  unlisteners.push(await onScriptSaved(() => store.initSavedScripts()))
  unlisteners.push(await onScriptsChanged(() => store.initSavedScripts()))

  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateLayoutMode)
  window.removeEventListener('keydown', handleKeydown)
  unlisteners.forEach((fn) => fn?.())
})

function requestExecute() {
  window.dispatchEvent(new CustomEvent('request-execute-rename'))
}

function handleKeydown(e) {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return

  if (e.ctrlKey && e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    if (canPreview.value) store.previewRename()
  } else if (e.ctrlKey && e.shiftKey && e.key === 'Enter') {
    e.preventDefault()
    window.dispatchEvent(new CustomEvent('request-execute-rename'))
  } else if (e.ctrlKey && e.key === 'z') {
    e.preventDefault()
    if (store.canUndo) store.undo()
  } else if (e.ctrlKey && e.key === 'o') {
    e.preventDefault()
    store.selectFiles()
  } else if (e.ctrlKey && e.key === 'r') {
    e.preventDefault()
    store.initSavedScripts()
  }
}

function startResize(target, e) {
  resizeTarget = target
  startX = e.clientX
  startWidth = target === 'left' ? leftWidth.value : middleWidth.value
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
  e.preventDefault()
}

function onResize(e) {
  if (!resizeTarget) return
  const delta = e.clientX - startX
  const newWidth = Math.max(220, Math.min(600, startWidth + delta))
  if (resizeTarget === 'left') {
    leftWidth.value = newWidth
  } else {
    middleWidth.value = newWidth
  }
}

function stopResize() {
  resizeTarget = null
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
}
</script>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #f5f5f5;
}

.app-header {
  padding: 12px 24px 8px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
  flex-shrink: 0;
}

.header-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.app-header h1 {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
  color: #333;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-divider {
  width: 1px;
  height: 24px;
  background: #ddd;
  margin: 0 4px;
}

.error-banner {
  margin-top: 8px;
  padding: 10px 14px;
  background: #fff3cd;
  border: 1px solid #ffc107;
  border-radius: 4px;
  color: #856404;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
}

.execute-progress-section {
  margin-top: 8px;
  padding: 8px 10px;
  background: #f0fff4;
  border: 1px solid #c3e6cb;
  border-radius: 6px;
}

.execute-progress-bar {
  height: 6px;
  background: #d4edda;
  border-radius: 3px;
  overflow: hidden;
}

.execute-progress-fill {
  height: 100%;
  background: #28a745;
  border-radius: 3px;
  transition: width 0.2s ease;
}

.execute-progress-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 6px;
  font-size: 12px;
  color: #555;
}

.execute-progress-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.execute-progress-count {
  flex-shrink: 0;
  font-weight: 600;
  color: #28a745;
}

.btn-close-error {
  background: none;
  border: none;
  font-size: 20px;
  color: #856404;
  cursor: pointer;
  padding: 0;
  width: 24px;
  height: 24px;
}

.app-content {
  flex: 1;
  display: flex;
  gap: 0;
  padding: 16px;
  overflow: hidden;
  min-height: 0;
}

.resizable-panel {
  position: relative;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.resize-handle {
  position: absolute;
  right: -4px;
  top: 0;
  bottom: 0;
  width: 8px;
  cursor: col-resize;
  z-index: 10;
}

.resize-handle:hover {
  background: rgba(0, 123, 255, 0.15);
}

.left-panel {
  margin-right: 8px;
}

.middle-panel {
  margin-right: 16px;
}

.right-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.panel-tabs {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
  padding-bottom: 8px;
}

.panel-tab {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 10px;
  border: 1px solid #ddd;
  border-radius: 6px;
  background: #fff;
  color: #555;
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s, border-color 0.15s, color 0.15s;
}

.panel-tab:hover {
  border-color: #007bff;
  color: #007bff;
}

.panel-tab.active {
  background: #007bff;
  border-color: #007bff;
  color: #fff;
  font-weight: 600;
}

.panel-tab-badge {
  min-width: 18px;
  padding: 0 6px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.12);
  font-size: 11px;
  line-height: 18px;
}

.panel-tab.active .panel-tab-badge {
  background: rgba(255, 255, 255, 0.25);
}

.app-content.narrow {
  flex-direction: column;
  padding: 8px 12px 12px;
  overflow: hidden;
}

.app-content.narrow .left-panel,
.app-content.narrow .middle-panel,
.app-content.narrow .right-panel {
  flex: 1;
  min-height: 0;
  width: 100% !important;
  margin: 0;
}

@media (max-width: 1100px) {
  .app-header {
    padding: 8px 12px 6px;
  }

  .header-top {
    flex-wrap: wrap;
    gap: 8px;
  }

  .app-header h1 {
    font-size: 18px;
  }

  .header-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .header-divider {
    display: none;
  }
}

@media (max-width: 640px) {
  .header-actions .btn {
    padding: 5px 8px;
    font-size: 12px;
  }
}
</style>
