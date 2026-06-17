<template>
  <div v-if="visible" class="dialog-overlay" @click.self="onCancel">
    <div class="dialog-content" role="dialog" aria-modal="true">
      <div class="dialog-header">
        <h4>{{ title }}</h4>
      </div>
      <div class="dialog-body">
        <p v-if="message" class="message">{{ message }}</p>
        <input
          ref="inputRef"
          v-model="localValue"
          type="text"
          class="prompt-input"
          :placeholder="placeholder"
          @keydown.enter="onConfirm"
          @keydown.esc="onCancel"
        />
      </div>
      <div class="dialog-footer">
        <button class="btn btn-secondary" @click="onCancel">取消</button>
        <button class="btn btn-primary" @click="onConfirm" :disabled="!localValue.trim()">确定</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'

const props = defineProps({
  visible: { type: Boolean, default: false },
  title: { type: String, default: '输入' },
  message: { type: String, default: '' },
  placeholder: { type: String, default: '' },
  defaultValue: { type: String, default: '' },
})

const emit = defineEmits(['confirm', 'cancel'])

const localValue = ref('')
const inputRef = ref(null)

watch(
  () => props.visible,
  async (v) => {
    if (v) {
      localValue.value = props.defaultValue
      await nextTick()
      inputRef.value?.focus()
      inputRef.value?.select()
    }
  }
)

const onConfirm = () => {
  const val = localValue.value.trim()
  if (val) emit('confirm', val)
}

const onCancel = () => emit('cancel')
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.dialog-content {
  background: white;
  border-radius: 8px;
  width: 90%;
  max-width: 420px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
}

.dialog-header {
  padding: 16px 20px;
  border-bottom: 1px solid #e0e0e0;
}

.dialog-header h4 {
  margin: 0;
  font-size: 16px;
}

.dialog-body {
  padding: 16px 20px;
}

.message {
  margin: 0 0 12px;
  font-size: 14px;
  color: #555;
}

.prompt-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
}

.prompt-input:focus {
  outline: none;
  border-color: #007bff;
}

.dialog-footer {
  padding: 12px 20px;
  border-top: 1px solid #e0e0e0;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.btn {
  padding: 6px 14px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-primary {
  background: #007bff;
  color: white;
}

.btn-secondary {
  background: #6c757d;
  color: white;
}
</style>
