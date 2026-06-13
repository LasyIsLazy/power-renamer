<template>
  <div class="tree-node" :style="{ paddingLeft: depth * 16 + 8 + 'px' }">
    <!-- 目录节点 -->
    <template v-if="node.type === 'dir'">
      <div class="tree-dir-row" @click="$emit('toggle', node.key)">
        <span class="tree-toggle">{{ expanded ? '▼' : '▶' }}</span>
        <span class="tree-icon tree-icon-dir">📁</span>
        <span class="tree-dir-name">{{ node.name }}</span>
      </div>
      <template v-if="expanded">
        <PreviewTreeNodeSelf
          v-for="(child, i) in node.children"
          :key="child.key + '-' + i"
          :node="child"
          :depth="depth + 1"
          :expanded-keys="expandedKeys"
          :get-path-parts="getPathParts"
          :has-changed="hasChanged"
          @toggle="$emit('toggle', $event)"
        />
      </template>
    </template>

    <!-- 文件节点：原路径 → 新路径（带高亮） -->
    <template v-else>
      <div class="tree-file-row" :class="{ error: node.result && node.result.error }">
        <span class="tree-icon tree-icon-file">📄</span>
        <div class="tree-file-content">
          <div class="tree-path-row">
            <span class="path-value">
              <template v-for="(part, i) in getPathParts(node.result.original, node.result.new_name).original" :key="'o-' + i">
                <span :class="part.type === 'common' ? 'path-common' : 'path-diff path-diff-removed'">{{ part.text }}</span>
              </template>
            </span>
            <span class="arrow">→</span>
            <span class="path-value" :class="{ changed: hasChanged(node.result) }">
              <template v-for="(part, i) in getPathParts(node.result.original, node.result.new_name).new" :key="'n-' + i">
                <span :class="part.type === 'common' ? 'path-common' : 'path-diff path-diff-added'">{{ part.text }}</span>
              </template>
            </span>
          </div>
          <div v-if="node.result.error" class="error-message">{{ node.result.error }}</div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import PreviewTreeNodeSelf from './PreviewTreeNode.vue'

const props = defineProps({
  node: { type: Object, required: true },
  depth: { type: Number, default: 0 },
  expandedKeys: { type: Set, default: () => new Set() },
  getPathParts: { type: Function, required: true },
  hasChanged: { type: Function, required: true },
})

defineEmits(['toggle'])

const expanded = computed(() => props.expandedKeys.has(props.node.key))
</script>

<style scoped>
.tree-node {
  min-width: 0;
}

.tree-dir-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  cursor: pointer;
  font-size: 13px;
  color: #555;
  user-select: none;
}

.tree-dir-row:hover {
  background: #f0f0f0;
  border-radius: 4px;
}

.tree-toggle {
  width: 14px;
  font-size: 10px;
  color: #888;
}

.tree-icon {
  font-size: 14px;
  line-height: 1;
}

.tree-dir-name {
  font-weight: 500;
}

.tree-file-row {
  padding: 6px 0;
  font-size: 13px;
  border-radius: 4px;
}

.tree-file-row:hover {
  background: #f8f8f8;
}

.tree-file-row.error {
  background: #fff5f5;
}

.tree-file-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.tree-path-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-family: Monaco, 'Courier New', monospace;
  font-size: 12px;
  word-break: break-all;
}

.tree-path-row .path-value {
  display: inline;
}

.tree-path-row .path-common {
  color: #666;
}

.tree-path-row .path-diff {
  font-weight: 600;
  padding: 0 1px;
  border-radius: 2px;
}

.tree-path-row .path-diff-removed {
  background: rgba(220, 53, 69, 0.15);
  color: #c82333;
}

.tree-path-row .path-diff-added {
  background: rgba(40, 167, 69, 0.2);
  color: #1e7e34;
}

.tree-path-row .path-value.changed {
  color: #1e7e34;
}

.tree-path-row .arrow {
  color: #999;
  flex-shrink: 0;
}

.error-message {
  font-size: 11px;
  color: #dc3545;
  padding-left: 22px;
}
</style>
