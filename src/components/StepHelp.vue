<template>
  <div class="step-help">
    <button class="help-toggle" @click="open = !open">
      <span class="help-icon">?</span>
      <span>{{ open ? "收起说明" : "查看此步骤说明" }}</span>
      <span class="arrow">{{ open ? "▲" : "▼" }}</span>
    </button>
    <div v-show="open" class="help-body">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
const open = ref(false);
</script>

<style scoped>
.step-help { margin-top: 4px; }
.help-toggle {
  display: flex; align-items: center; gap: 6px;
  background: none; border: 1px dashed var(--color-border);
  border-radius: var(--radius); padding: 5px 12px;
  font-size: 12px; color: var(--color-muted); cursor: pointer;
  width: 100%;
}
.help-toggle:hover { border-color: var(--color-primary); color: var(--color-primary); }
.help-icon {
  display: inline-flex; align-items: center; justify-content: center;
  width: 16px; height: 16px; border-radius: 50%;
  border: 1px solid currentColor; font-size: 10px; font-weight: 700;
  flex-shrink: 0;
}
.arrow { margin-left: auto; font-size: 10px; }
.help-body {
  margin-top: 8px;
  background: #f8fafc; border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 14px 16px;
  font-size: 13px; line-height: 1.8; color: #475569;
}
.help-body :deep(ul) { padding-left: 20px; margin: 6px 0; }
.help-body :deep(li) { margin-bottom: 4px; }
.help-body :deep(strong) { color: #1e293b; }
.help-body :deep(.tip) {
  background: #eff6ff; border-left: 3px solid #3b82f6;
  padding: 8px 12px; border-radius: 0 4px 4px 0; margin-top: 10px;
}
</style>
