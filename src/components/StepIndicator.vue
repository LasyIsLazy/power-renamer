<template>
  <div class="step-indicator">
    <div
      v-for="step in steps"
      :key="step.id"
      class="step"
      :class="{
        active: currentStep === step.id,
        completed: currentStep > step.id,
      }"
      :title="step.hint"
    >
      <span class="step-num">{{ step.id }}</span>
      <span class="step-label">{{ step.label }}</span>
      <span v-if="step.id < steps.length" class="step-arrow">→</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useRenameStore } from '../stores/renameStore'

const store = useRenameStore()

const steps = [
  { id: 1, label: '选择文件', hint: '选择或拖放待重命名的文件/文件夹' },
  { id: 2, label: '选择脚本', hint: '从列表选择脚本或快速规则' },
  { id: 3, label: '预览', hint: '点击预览查看重命名结果' },
  { id: 4, label: '执行', hint: '确认后执行重命名' },
]

const currentStep = computed(() => store.workflowStep)
</script>

<style scoped>
.step-indicator {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 0 0;
  flex-wrap: wrap;
}

.step {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: #999;
}

.step.active {
  color: #007bff;
  font-weight: 600;
}

.step.completed {
  color: #28a745;
}

.step-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #e9ecef;
  font-size: 12px;
  font-weight: 600;
}

.step.active .step-num {
  background: #007bff;
  color: white;
}

.step.completed .step-num {
  background: #28a745;
  color: white;
}

.step-arrow {
  margin: 0 8px;
  color: #ccc;
}
</style>
