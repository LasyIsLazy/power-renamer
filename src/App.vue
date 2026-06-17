<template>
  <div class="app">
    <div class="app-header">
      <div class="header-top">
        <h1>Power Renamer</h1>
        <div class="header-actions">
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
      <div v-if="store.error" class="error-banner">
        <span>{{ store.error }}</span>
        <button @click="store.clearError" class="btn-close-error">×</button>
      </div>
    </div>

    <div class="app-content">
      <div class="left-panel resizable-panel" :style="{ width: leftWidth + 'px' }">
        <FileSelector />
        <div class="resize-handle" @mousedown="startResize('left', $event)"></div>
      </div>

      <div class="middle-panel resizable-panel" :style="{ width: middleWidth + 'px' }">
        <ScriptList />
        <div class="resize-handle" @mousedown="startResize('middle', $event)"></div>
      </div>

      <div class="right-panel">
        <PreviewPanel />
      </div>
    </div>

    <HistoryPanel :drawer="true" />
  </div>
</template>

<script setup>
import { onMounted, onUnmounted, ref } from 'vue'
import { useRenameStore } from './stores/renameStore'
import FileSelector from './components/FileSelector.vue'
import ScriptList from './components/ScriptList.vue'
import PreviewPanel from './components/PreviewPanel.vue'
import HistoryPanel from './components/HistoryPanel.vue'
import StepIndicator from './components/StepIndicator.vue'
import { onScriptSaved, onScriptsChanged } from './composables/scriptEditor'

const store = useRenameStore()
const leftWidth = ref(300)
const middleWidth = ref(400)
let resizeTarget = null
let startX = 0
let startWidth = 0
let unlisteners = []

onMounted(async () => {
  await store.initSavedScripts()
  await store.initScriptLogs()
  await store.initHistory()

  unlisteners.push(await onScriptSaved(() => store.initSavedScripts()))
  unlisteners.push(await onScriptsChanged(() => store.initSavedScripts()))

  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  unlisteners.forEach((fn) => fn?.())
})

function handleKeydown(e) {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return

  if (e.ctrlKey && e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    store.previewRename()
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
  gap: 8px;
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

@media (max-width: 1200px) {
  .app-content {
    flex-direction: column;
    overflow-y: auto;
  }

  .resizable-panel {
    width: 100% !important;
    min-height: 280px;
  }

  .resize-handle {
    display: none;
  }
}
</style>
