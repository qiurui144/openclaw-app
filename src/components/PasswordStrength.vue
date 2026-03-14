<template>
  <div class="strength-bar" v-if="password.length > 0">
    <div class="track">
      <div class="fill" :class="level" :style="{ width: pct + '%' }" />
    </div>
    <span class="label">{{ labels[level] }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ password: string }>();

const score = computed(() => {
  const p = props.password;
  let s = 0;
  if (p.length >= 8) s++;
  if (p.length >= 12) s++;
  if (/[A-Z]/.test(p)) s++;
  if (/[0-9]/.test(p)) s++;
  if (/[^a-zA-Z0-9]/.test(p)) s++;
  return s;
});

const level = computed<"weak" | "medium" | "strong">(() => {
  if (score.value <= 2) return "weak";
  if (score.value <= 3) return "medium";
  return "strong";
});

const pct = computed(() => Math.min(100, (score.value / 5) * 100));

const labels = { weak: "弱", medium: "中", strong: "强" };
</script>

<style scoped>
.strength-bar { display: flex; align-items: center; gap: 8px; margin-top: 6px; }
.track { flex: 1; height: 4px; background: var(--color-border); border-radius: 2px; overflow: hidden; }
.fill { height: 100%; border-radius: 2px; transition: width .3s; }
.fill.weak   { background: var(--color-error); }
.fill.medium { background: var(--color-warning); }
.fill.strong { background: var(--color-success); }
.label { font-size: 12px; color: var(--color-muted); min-width: 16px; }
</style>
