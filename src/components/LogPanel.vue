<template>
  <div class="log-panel">
    <button class="toggle" @click="open = !open">
      {{ open ? "▲ 折叠日志" : "▼ 展开日志" }}
    </button>
    <div v-show="open" class="log-body" ref="logEl">
      <div v-for="(line, i) in logs" :key="i" class="log-line">{{ line }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from "vue";

const props = defineProps<{ logs: string[] }>();
const open = ref(true);
const logEl = ref<HTMLDivElement | null>(null);

watch(() => props.logs.length, async () => {
  if (open.value) {
    await nextTick();
    logEl.value?.scrollTo({ top: logEl.value.scrollHeight, behavior: "smooth" });
  }
});
</script>

<style scoped>
.log-panel { margin-top: 16px; }
.toggle {
  background: none; border: 1px solid var(--color-border);
  font-size: 12px; color: var(--color-muted); padding: 4px 10px; width: 100%;
}
.log-body {
  margin-top: 4px;
  max-height: 180px; overflow-y: auto;
  background: #0f172a; color: #94a3b8;
  border-radius: var(--radius);
  padding: 12px;
}
.log-line { font-family: monospace; font-size: 12px; line-height: 1.6; }
</style>
