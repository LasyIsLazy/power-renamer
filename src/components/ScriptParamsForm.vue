<template>
  <div v-if="parameters && parameters.length > 0" class="script-params">
    <div v-if="collapsible" class="params-header" @click="$emit('toggle')">
      <span>{{ headerText }}</span>
      <span class="params-toggle">{{ expanded ? '▼' : '▶' }}</span>
    </div>
    <div v-if="!collapsible || expanded" class="params-content">
      <div v-for="param in parameters" :key="param.name" class="param-item">
        <label class="param-label">
          {{ param.name }}
          <span v-if="param.required" class="required">*</span>
          <span v-if="param.description" class="param-desc">({{ param.description }})</span>
        </label>
        <input
          v-if="param.type === 'string'"
          type="text"
          :value="getValue(param.name)"
          @input="setValue(param.name, $event.target.value)"
          class="param-input"
          :class="{ error: isInvalid(param) }"
        />
        <input
          v-else-if="param.type === 'number'"
          type="number"
          :value="getValue(param.name)"
          @input="setValue(param.name, parseFloat($event.target.value) || 0)"
          class="param-input"
          :class="{ error: isInvalid(param) }"
        />
        <label v-else-if="param.type === 'boolean'" class="param-checkbox">
          <input
            type="checkbox"
            :checked="getValue(param.name)"
            @change="setValue(param.name, $event.target.checked)"
          />
          <span>{{ param.description || param.name }}</span>
        </label>
      </div>
    </div>
  </div>
</template>

<script setup>
import { useRenameStore } from '../stores/renameStore'

const props = defineProps({
  scriptId: { type: String, required: true },
  parameters: { type: Array, default: () => [] },
  expanded: { type: Boolean, default: true },
  collapsible: { type: Boolean, default: true },
  headerText: { type: String, default: '脚本参数' },
})

defineEmits(['toggle'])

const store = useRenameStore()

const getValue = (paramName) => {
  const params = store.scriptParams[props.scriptId]
  return params ? params[paramName] : undefined
}

const setValue = (paramName, value) => {
  store.setScriptParam(props.scriptId, paramName, value)
}

const isInvalid = (param) => {
  if (!param.required) return false
  const value = getValue(param.name)
  if (value === undefined || value === null || value === '') return true
  if (param.type === 'number' && isNaN(value)) return true
  return false
}
</script>

<style scoped>
.script-params {
  padding: 12px 16px;
  background: #f9f9f9;
  border-bottom: 1px solid #e0e0e0;
}

.params-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  cursor: pointer;
  font-size: 13px;
  color: #666;
  font-weight: 500;
}

.params-header:hover {
  color: #007bff;
}

.params-toggle {
  font-size: 12px;
  color: #999;
}

.params-content {
  padding: 8px 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.param-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.param-label {
  font-size: 13px;
  color: #333;
  font-weight: 500;
}

.param-label .required {
  color: #dc3545;
  margin-left: 2px;
}

.param-label .param-desc {
  color: #666;
  font-weight: normal;
  font-size: 12px;
  margin-left: 4px;
}

.param-input {
  padding: 6px 8px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 13px;
  width: 100%;
}

.param-input:focus {
  outline: none;
  border-color: #007bff;
}

.param-input.error {
  border-color: #dc3545;
}

.param-checkbox {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  cursor: pointer;
}
</style>
