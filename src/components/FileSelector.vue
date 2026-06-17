<template>
  <div class="file-selector panel-card">
    <div class="panel-header">
      <h3>文件列表 <span v-if="store.hasFiles" class="file-count">({{ fileCountLabel }})</span></h3>
      <div class="actions">
        <button @click="selectFiles" class="btn btn-primary" title="选择文件 (Ctrl+O)">选择文件</button>
        <button @click="selectFolder" class="btn btn-secondary">选择文件夹</button>
        <button v-if="store.hasFiles" @click="store.clearFiles" class="btn btn-danger">清空</button>
      </div>
    </div>

    <div
      v-if="!store.hasFiles"
      class="drop-zone"
      :class="{ 'drop-zone-active': isDragOver }"
    >
      <div class="drop-zone-content">
        <p class="step-hint">步骤 1：拖拽文件或文件夹到这里</p>
        <p class="hint">或点击上方按钮选择</p>
        <p class="hint">支持多文件追加；文件夹一次只能选一个</p>
      </div>
    </div>

    <div v-else class="file-list">
      <div v-for="(file, index) in store.files" :key="index" class="file-item">
        <div class="file-icon">
          <svg v-if="store.basePath" width="20" height="20" viewBox="0 0 24 24" fill="#ffc107" stroke="#e0a800" stroke-width="1"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
          <svg v-else width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
        </div>
        <div class="file-info">
          <span class="file-name">{{ file.name }}</span>
          <span class="file-path">{{ file.path }}</span>
        </div>
        <button @click="store.removeFile(index)" class="btn-remove" title="移除">×</button>
      </div>
    </div>

    <div v-if="store.basePath" class="base-path">
      <div>文件夹模式: {{ store.basePath }}</div>
      <div v-if="store.folderFileCount !== null" class="folder-stats">
        包含 {{ store.folderFileCount }} 个文件（递归统计）
        <span v-if="store.folderFileCount === 0" class="empty-folder">（空文件夹）</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRenameStore } from '../stores/renameStore'

const store = useRenameStore()
const isDragOver = ref(false)
let unlistenDragDrop = null

const fileCountLabel = computed(() => {
  if (store.basePath) {
    return store.folderFileCount !== null
      ? `1 个文件夹，${store.folderFileCount} 个文件`
      : '1 个文件夹'
  }
  return `${store.files.length} 个文件`
})

const selectFiles = () => store.selectFiles()
const selectFolder = () => store.selectFolder()

onMounted(async () => {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const appWindow = getCurrentWindow()
    unlistenDragDrop = await appWindow.onDragDropEvent((event) => {
      const { type } = event.payload
      if (type === 'over') {
        isDragOver.value = true
      } else if (type === 'drop') {
        isDragOver.value = false
        const paths = event.payload.paths
        if (paths?.length > 0) store.addDroppedPaths(paths)
      } else {
        isDragOver.value = false
      }
    })
  } catch (error) {
    console.warn('Tauri 拖放 API 不可用:', error)
  }
})

onUnmounted(() => {
  if (unlistenDragDrop) {
    unlistenDragDrop()
    unlistenDragDrop = null
  }
})
</script>

<style scoped>
.file-count {
  font-size: 13px;
  font-weight: 400;
  color: #666;
}

.drop-zone {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px dashed #ccc;
  margin: 16px;
  border-radius: 8px;
  background: #fafafa;
  transition: border-color 0.2s, background-color 0.2s;
}

.drop-zone:hover,
.drop-zone-active {
  border-color: #007bff;
  background: #f0f7ff;
}

.drop-zone-content { text-align: center; color: #666; pointer-events: none; }
.step-hint { font-weight: 500; color: #333; margin: 8px 0; }
.hint { font-size: 12px; color: #999; margin: 4px 0; }

.file-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  margin-bottom: 4px;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
}

.file-item:hover { background: #f5f5f5; }

.file-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.file-name { font-weight: 500; color: #333; }
.file-path {
  font-size: 12px;
  color: #666;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-remove {
  background: none;
  border: none;
  font-size: 24px;
  color: #dc3545;
  cursor: pointer;
  padding: 0 8px;
}

.base-path {
  padding: 8px 16px;
  background: #e3f2fd;
  border-top: 1px solid #e0e0e0;
  font-size: 12px;
  color: #1976d2;
}

.folder-stats { margin-top: 4px; }
.empty-folder { color: #dc3545; }
</style>
