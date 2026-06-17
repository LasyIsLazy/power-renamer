<template>
  <div v-if="visible" class="dialog-overlay" @click.self="onCancel">
    <div class="dialog-content" role="dialog" aria-modal="true">
      <div class="dialog-header">
        <h4>{{ title }}</h4>
      </div>
      <div class="dialog-body">
        <p v-if="message">{{ message }}</p>
        <slot />
      </div>
      <div class="dialog-footer">
        <button class="btn btn-secondary" @click="onCancel">{{ cancelText }}</button>
        <button
          class="btn"
          :class="confirmVariant === 'danger' ? 'btn-danger' : 'btn-primary'"
          @click="onConfirm"
        >
          {{ confirmText }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
defineProps({
  visible: { type: Boolean, default: false },
  title: { type: String, default: '确认' },
  message: { type: String, default: '' },
  confirmText: { type: String, default: '确定' },
  cancelText: { type: String, default: '取消' },
  confirmVariant: { type: String, default: 'primary' },
})

const emit = defineEmits(['confirm', 'cancel'])

const onConfirm = () => emit('confirm')
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
  max-width: 480px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  overflow: hidden;
}

.dialog-header {
  padding: 16px 20px;
  border-bottom: 1px solid #e0e0e0;
}

.dialog-header h4 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.dialog-body {
  padding: 16px 20px;
  color: #333;
  font-size: 14px;
  line-height: 1.5;
}

.dialog-body p {
  margin: 0;
  white-space: pre-wrap;
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
  background: #5a6268;
}

.btn-danger {
  background: #dc3545;
  color: white;
}

.btn-danger:hover {
  background: #c82333;
}
</style>
