<template>
  <div class="file-selector">
    <div class="header">
      <h3>文件列表</h3>
      <div class="actions">
        <button @click="selectFiles" class="btn btn-primary">选择文件</button>
        <button @click="selectFolder" class="btn btn-secondary">选择文件夹</button>
        <button
          v-if="store.hasFiles"
          @click="store.clearFiles"
          class="btn btn-danger"
        >
          清空
        </button>
      </div>
    </div>

    <div
      v-if="!store.hasFiles"
      class="drop-zone"
      :class="{ 'drop-zone-active': isDragOver }"
    >
      <div class="drop-zone-content">
        <p>拖拽文件或文件夹到这里</p>
        <p class="hint">或点击上方按钮选择</p>
      </div>
    </div>

    <div v-else class="file-list">
      <div
        v-for="(file, index) in store.files"
        :key="index"
        class="file-item"
      >
        <div class="file-info">
          <span class="file-name">{{ file.name }}</span>
          <span class="file-path">{{ file.path }}</span>
        </div>
        <button
          @click="store.removeFile(index)"
          class="btn-remove"
          title="移除"
        >
          ×
        </button>
      </div>
    </div>

    <div v-if="store.basePath" class="base-path">
      基础路径: {{ store.basePath }}
    </div>
  </div>
</template>

<script setup>
import { onMounted, onUnmounted, ref } from 'vue'
import { useRenameStore } from '../stores/renameStore'

const store = useRenameStore()
const isDragOver = ref(false)
let unlistenDragDrop = null

const selectFiles = () => {
  store.selectFiles()
}

const selectFolder = () => {
  store.selectFolder()
}

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
        if (paths && paths.length > 0) {
          store.addDroppedPaths(paths)
        }
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
.file-selector {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
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

.btn-danger {
  background: #dc3545;
  color: white;
}

.btn-danger:hover {
  background: #c82333;
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

.drop-zone-content {
  text-align: center;
  color: #666;
  pointer-events: none;
}

.drop-zone-content p {
  margin: 8px 0;
}

.hint {
  font-size: 12px;
  color: #999;
}

.file-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px;
}

.file-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  margin-bottom: 4px;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.file-item:hover {
  background: #f5f5f5;
}

.file-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.file-name {
  font-weight: 500;
  color: #333;
}

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
  line-height: 1;
  transition: color 0.2s;
}

.btn-remove:hover {
  color: #c82333;
}

.base-path {
  padding: 8px 16px;
  background: #e3f2fd;
  border-top: 1px solid #e0e0e0;
  font-size: 12px;
  color: #1976d2;
}
</style>
