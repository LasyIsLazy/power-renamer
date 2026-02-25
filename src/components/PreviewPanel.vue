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

    <div v-if="store.loading" class="loading">
      <p>正在预览...</p>
    </div>

    <div v-else-if="!store.hasPreview" class="empty">
      <p>点击"预览"按钮查看重命名结果</p>
    </div>

    <div v-else class="preview-list">
      <div
        v-for="(result, index) in store.previewResults"
        :key="index"
        class="preview-item"
        :class="{ error: result.error }"
      >
        <div class="preview-info">
          <div class="original">
            <span class="label">原文件名:</span>
            <span class="value">{{ result.original }}</span>
          </div>
          <div class="arrow">→</div>
          <div class="new">
            <span class="label">新文件名:</span>
            <span class="value" :class="{ changed: hasChanged(result) }">
              {{ result.new_name }}
            </span>
          </div>
        </div>
        <div v-if="result.error" class="error-message">
          {{ result.error }}
        </div>
      </div>
    </div>

    <div v-if="store.hasPreview" class="footer">
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
import { computed } from 'vue'
import { useRenameStore } from '../stores/renameStore'

const store = useRenameStore()

const canPreview = computed(() => {
  return store.hasFiles && store.scriptValid
})

const changedCount = computed(() => {
  return store.previewResults.filter((r) => hasChanged(r)).length
})

const hasChanged = (result) => {
  return result.original !== result.new_name && !result.error
}
</script>

<style scoped>
.preview-panel {
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
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
}

.preview-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
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
}

.stats {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: #666;
}
</style>
