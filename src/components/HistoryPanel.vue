<template>
  <div class="history-drawer" :class="{ open: store.showHistoryDrawer }">
    <div class="drawer-overlay" @click="store.showHistoryDrawer = false"></div>
    <div class="drawer-panel">
      <div class="panel-header">
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
            @click="showClearConfirm = true"
            class="btn btn-danger"
          >
            清空
          </button>
          <button @click="store.showHistoryDrawer = false" class="btn-icon" title="关闭">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
      </div>

      <div v-if="store.history.length === 0" class="empty-state">
        <p>暂无历史记录</p>
        <p class="hint">执行重命名后，记录将显示在这里</p>
      </div>

      <div v-else class="history-list">
        <div v-for="(item, index) in store.history" :key="index" class="history-item">
          <div class="history-header">
            <span class="timestamp">{{ formatTime(item.timestamp) }}</span>
            <span class="file-count">{{ item.files.length }} 个文件</span>
          </div>
          <div class="history-script">
            <code>{{ truncateScript(item.script) }}</code>
          </div>
          <div class="history-results">
            <span class="success-count">成功: {{ item.results.filter((r) => r.success).length }}</span>
            <span class="error-count">失败: {{ item.results.filter((r) => !r.success).length }}</span>
          </div>
          <div class="history-actions">
            <button @click="loadHistory(item)" class="btn-link">加载脚本</button>
            <button @click="viewDetails(item)" class="btn-link">查看详情</button>
          </div>
        </div>
      </div>
    </div>

    <div v-if="showDetails" class="modal" @click.self="showDetails = false">
      <div class="modal-content">
        <div class="modal-header">
          <h4>历史记录详情</h4>
          <button @click="showDetails = false" class="btn-icon">×</button>
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
              <li v-for="(file, idx) in selectedHistory.files" :key="idx">{{ file.name }}</li>
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
                <strong>{{ result.original }}</strong> → <span>{{ result.new_name }}</span>
              </div>
              <div v-if="result.error" class="error-text">{{ result.error }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <ConfirmDialog
      :visible="showClearConfirm"
      title="清空历史"
      message="确定要清空所有历史记录吗？撤销栈也将被清空。"
      confirm-text="清空"
      confirm-variant="danger"
      @confirm="confirmClear"
      @cancel="showClearConfirm = false"
    />
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRenameStore } from '../stores/renameStore'
import ConfirmDialog from './ui/ConfirmDialog.vue'

defineProps({
  drawer: { type: Boolean, default: false },
})

const store = useRenameStore()
const showDetails = ref(false)
const showClearConfirm = ref(false)
const selectedHistory = ref(null)

const formatTime = (timestamp) => new Date(timestamp).toLocaleString('zh-CN')

const truncateScript = (script) =>
  script.length > 50 ? script.substring(0, 50) + '...' : script

const loadHistory = (item) => {
  store.updateScript(item.script)
  store.showHistoryDrawer = false
}

const viewDetails = (item) => {
  selectedHistory.value = item
  showDetails.value = true
}

const confirmClear = () => {
  showClearConfirm.value = false
  store.clearHistory()
}
</script>

<style scoped>
.history-drawer {
  position: fixed;
  inset: 0;
  z-index: 1500;
  pointer-events: none;
}

.history-drawer.open {
  pointer-events: auto;
}

.drawer-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  opacity: 0;
  transition: opacity 0.25s;
}

.history-drawer.open .drawer-overlay {
  opacity: 1;
}

.drawer-panel {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 400px;
  max-width: 90vw;
  background: white;
  box-shadow: -4px 0 16px rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
  transform: translateX(100%);
  transition: transform 0.25s;
}

.history-drawer.open .drawer-panel {
  transform: translateX(0);
}

.hint { font-size: 12px; color: #999; margin-top: 8px; }

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
}

.history-item:hover { border-color: #007bff; }

.history-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
  font-size: 12px;
}

.timestamp { color: #666; }
.file-count { color: #007bff; font-weight: 500; }

.history-script {
  margin-bottom: 8px;
  padding: 8px;
  background: #f5f5f5;
  border-radius: 4px;
}

.history-script code { font-family: Monaco, monospace; font-size: 12px; }

.history-results {
  display: flex;
  gap: 16px;
  margin-bottom: 8px;
  font-size: 12px;
}

.success-count { color: #28a745; }
.error-count { color: #dc3545; }

.history-actions { display: flex; gap: 12px; }

.btn-link {
  background: none;
  border: none;
  color: #007bff;
  cursor: pointer;
  font-size: 12px;
  text-decoration: underline;
  padding: 0;
}

.modal {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1600;
  pointer-events: auto;
}

.modal-content {
  background: white;
  border-radius: 8px;
  width: 90%;
  max-width: 800px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid #e0e0e0;
}

.modal-header h4 { margin: 0; }

.modal-body { flex: 1; overflow-y: auto; padding: 16px; }

.detail-section { margin-bottom: 24px; }
.detail-section h5 { margin: 0 0 8px; font-size: 14px; color: #666; }
.detail-section pre { background: #f5f5f5; padding: 12px; border-radius: 4px; overflow-x: auto; }
.detail-section ul { list-style: none; padding: 0; margin: 0; }
.detail-section li { padding: 4px 0; border-bottom: 1px solid #f0f0f0; }

.result-item { padding: 8px; margin-bottom: 4px; border-radius: 4px; }
.result-item.success { background: #d4edda; }
.result-item.error { background: #f8d7da; }
.error-text { margin-top: 4px; font-size: 12px; color: #dc3545; }
</style>
