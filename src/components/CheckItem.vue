<template>
  <div class="check-item" :class="item.status">
    <span class="icon">{{ statusIcon }}</span>
    <div class="info">
      <span class="label">{{ item.label }}</span>
      <span v-if="item.detail" class="detail">{{ item.detail }}</span>
    </div>
    <span class="badge" v-if="item.required && item.status === 'error'">必需</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { CheckItem } from "@/stores/wizard";

const props = defineProps<{ item: CheckItem }>();

const statusIcon = computed(() => ({
  pending: "○", running: "⏳", ok: "✓", warn: "⚠️", error: "✕"
}[props.item.status]));
</script>

<style scoped>
.check-item {
  display: flex; align-items: flex-start; gap: 12px;
  padding: 12px 16px;
  border-radius: var(--radius);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
}
.check-item.ok { border-color: #bbf7d0; background: #f0fdf4; }
.check-item.warn { border-color: #fde68a; background: #fffbeb; }
.check-item.error { border-color: #fecaca; background: #fef2f2; }

.icon { font-size: 16px; margin-top: 1px; }
.info { flex: 1; }
.label { font-weight: 500; }
.detail { display: block; font-size: 12px; color: var(--color-muted); margin-top: 2px; }
.badge {
  font-size: 11px; padding: 2px 6px;
  background: var(--color-error); color: #fff;
  border-radius: 4px;
}
</style>
