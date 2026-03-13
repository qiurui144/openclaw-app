<template>
  <div class="steps">
    <template v-for="(step, i) in steps" :key="i">
      <div
        class="step-dot"
        :class="{
          done: i < current,
          active: i === current,
          upcoming: i > current,
        }"
        :title="step.label"
      >
        <span v-if="i < current">✓</span>
        <span v-else>{{ i + 1 }}</span>
      </div>
      <div v-if="i < steps.length - 1" class="step-line" :class="{ done: i < current }" />
    </template>
  </div>
</template>

<script setup lang="ts">
defineProps<{ steps: { label: string }[]; current: number }>();
</script>

<style scoped>
.steps { display: flex; align-items: center; flex: 1; justify-content: center; }

.step-dot {
  width: 24px; height: 24px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 600;
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  color: var(--color-muted);
  flex-shrink: 0;
}
.step-dot.active { border-color: var(--color-primary); color: var(--color-primary); }
.step-dot.done { border-color: var(--color-success); background: var(--color-success); color: #fff; }

.step-line { flex: 1; height: 2px; background: var(--color-border); }
.step-line.done { background: var(--color-success); }
</style>
