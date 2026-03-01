<template>
  <div class="app">
    <div class="app-header">
      <h1>Power Renamer</h1>
      <div v-if="store.error" class="error-banner">
        <span>{{ store.error }}</span>
        <button @click="store.clearError" class="btn-close-error">×</button>
      </div>
    </div>

    <div class="app-content">
      <div class="left-panel">
        <FileSelector />
      </div>

      <div class="middle-panel">
        <ScriptEditor />
      </div>

      <div class="right-panel">
        <div class="right-top">
          <PreviewPanel />
        </div>
        <div class="right-bottom">
          <HistoryPanel />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { onMounted } from 'vue'
import { useRenameStore } from './stores/renameStore'
import FileSelector from './components/FileSelector.vue'
import ScriptEditor from './components/ScriptEditor.vue'
import PreviewPanel from './components/PreviewPanel.vue'
import HistoryPanel from './components/HistoryPanel.vue'

const store = useRenameStore()

// 初始化时加载保存的脚本
onMounted(() => {
  store.initSavedScripts()
})
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
  padding: 16px 24px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
}

.app-header h1 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: #333;
}

.error-banner {
  margin-top: 12px;
  padding: 12px 16px;
  background: #fff3cd;
  border: 1px solid #ffc107;
  border-radius: 4px;
  color: #856404;
  display: flex;
  justify-content: space-between;
  align-items: center;
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
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

.btn-close-error:hover {
  color: #533f03;
}

.app-content {
  flex: 1;
  display: flex;
  gap: 16px;
  padding: 16px;
  overflow: hidden;
}

.left-panel {
  flex: 0 0 300px;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.middle-panel {
  flex: 0 0 400px;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.right-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

.right-top {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.right-bottom {
  flex: 0 0 250px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

@media (max-width: 1200px) {
  .app-content {
    flex-direction: column;
  }

  .left-panel,
  .middle-panel,
  .right-panel {
    flex: 1;
  }

  .right-bottom {
    flex: 0 0 200px;
  }
}
</style>
