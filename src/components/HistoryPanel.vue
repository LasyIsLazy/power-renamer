<template>
  <div class="history-panel">
    <div class="header">
      <h3>历史记录</h3>
      <div class="actions">
        <button
          v-if="store.canUndo"
          @click="store.undo"
          :disabled="store.loading"
          class="btn btn-warning"
        >
          撤销
        </button>
        <button
          v-if="store.history.length > 0"
          @click="clearHistory"
          class="btn btn-danger"
        >
          清空历史
        </button>
      </div>
    </div>

    <div v-if="store.history.length === 0" class="empty">
      <p>暂无历史记录</p>
    </div>

    <div v-else class="history-list">
      <div
        v-for="(item, index) in store.history"
        :key="index"
        class="history-item"
      >
        <div class="history-header">
          <span class="timestamp">{{ formatTime(item.timestamp) }}</span>
          <span class="file-count">{{ item.files.length }} 个文件</span>
        </div>
        <div class="history-script">
          <code>{{ truncateScript(item.script) }}</code>
        </div>
        <div class="history-results">
          <span class="success-count">
            成功: {{ item.results.filter((r) => r.success).length }}
          </span>
          <span class="error-count">
            失败: {{ item.results.filter((r) => !r.success).length }}
          </span>
        </div>
        <div class="history-actions">
          <button @click="loadHistory(item)" class="btn-link">加载脚本</button>
          <button @click="viewDetails(item)" class="btn-link">查看详情</button>
        </div>
      </div>
    </div>

    <!-- 详情对话框 -->
    <div v-if="showDetails" class="modal" @click.self="showDetails = false">
      <div class="modal-content">
        <div class="modal-header">
          <h4>历史记录详情</h4>
          <button @click="showDetails = false" class="btn-close">×</button>
        </div>
        <div class="modal-body" v-if="selectedHistory">
          <div class="detail-section">
            <h5>时间</h5>
            <p>{{ formatTime(selectedHistory.timestamp) }}</p>
          </div>
          <div class="detail-section">
            <h5>脚本</h5>
            <pre><code>{{ selectedHistory.script }}</code></pre>
          </div>
          <div class="detail-section">
            <h5>文件列表</h5>
            <ul>
              <li v-for="(file, idx) in selectedHistory.files" :key="idx">
                {{ file.name }}
              </li>
            </ul>
          </div>
          <div class="detail-section">
            <h5>执行结果</h5>
            <div
              v-for="(result, idx) in selectedHistory.results"
              :key="idx"
              class="result-item"
              :class="{ success: result.success, error: !result.success }"
            >
              <div>
                <strong>{{ result.original }}</strong> →
                <span>{{ result.new_name }}</span>
              </div>
              <div v-if="result.error" class="error-text">
                {{ result.error }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRenameStore } from '../stores/renameStore'

const store = useRenameStore()
const showDetails = ref(false)
const selectedHistory = ref(null)

const formatTime = (timestamp) => {
  const date = new Date(timestamp)
  return date.toLocaleString('zh-CN')
}

const truncateScript = (script) => {
  if (script.length > 50) {
    return script.substring(0, 50) + '...'
  }
  return script
}

const loadHistory = (item) => {
  store.updateScript(item.script)
}

const viewDetails = (item) => {
  selectedHistory.value = item
  showDetails.value = true
}

const clearHistory = () => {
  if (confirm('确定要清空所有历史记录吗？')) {
    store.history = []
    store.undoStack = []
  }
}
</script>

<style scoped>
.history-panel {
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

.btn-warning {
  background: #ffc107;
  color: #000;
}

.btn-warning:hover:not(:disabled) {
  background: #e0a800;
}

.btn-danger {
  background: #dc3545;
  color: white;
}

.btn-danger:hover {
  background: #c82333;
}

.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
}

.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.history-item {
  padding: 12px;
  margin-bottom: 8px;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  transition: border-color 0.2s;
}

.history-item:hover {
  border-color: #007bff;
}

.history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.timestamp {
  font-size: 12px;
  color: #666;
}

.file-count {
  font-size: 12px;
  color: #007bff;
  font-weight: 500;
}

.history-script {
  margin-bottom: 8px;
  padding: 8px;
  background: #f5f5f5;
  border-radius: 4px;
}

.history-script code {
  font-family: Monaco, 'Courier New', monospace;
  font-size: 12px;
  color: #333;
}

.history-results {
  display: flex;
  gap: 16px;
  margin-bottom: 8px;
  font-size: 12px;
}

.success-count {
  color: #28a745;
}

.error-count {
  color: #dc3545;
}

.history-actions {
  display: flex;
  gap: 12px;
}

.btn-link {
  background: none;
  border: none;
  color: #007bff;
  cursor: pointer;
  font-size: 12px;
  text-decoration: underline;
  padding: 0;
}

.btn-link:hover {
  color: #0056b3;
}

.modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: white;
  border-radius: 8px;
  width: 90%;
  max-width: 800px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid #e0e0e0;
}

.modal-header h4 {
  margin: 0;
}

.btn-close {
  background: none;
  border: none;
  font-size: 24px;
  cursor: pointer;
  color: #666;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-close:hover {
  color: #333;
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.detail-section {
  margin-bottom: 24px;
}

.detail-section h5 {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: #666;
}

.detail-section pre {
  background: #f5f5f5;
  padding: 12px;
  border-radius: 4px;
  overflow-x: auto;
}

.detail-section code {
  font-family: Monaco, 'Courier New', monospace;
  font-size: 12px;
}

.detail-section ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

.detail-section li {
  padding: 4px 0;
  border-bottom: 1px solid #f0f0f0;
}

.result-item {
  padding: 8px;
  margin-bottom: 4px;
  border-radius: 4px;
}

.result-item.success {
  background: #d4edda;
}

.result-item.error {
  background: #f8d7da;
}

.error-text {
  margin-top: 4px;
  font-size: 12px;
  color: #dc3545;
}
</style>
