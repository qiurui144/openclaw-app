<template>
  <div class="wizard-shell">
    <header class="wizard-header">
      <div class="brand">
        <span class="brand-icon">🦾</span>
        <span class="brand-name">OpenClaw 部署向导</span>
      </div>
      <StepIndicator :steps="visibleSteps" :current="currentIndex()" />
    </header>

    <main class="wizard-body">
      <slot />
    </main>

    <footer class="wizard-footer" v-if="showFooter">
      <button class="btn-secondary" @click="back()" :disabled="!canGoBack">
        ← 上一步
      </button>
      <span class="spacer" />
      <button
        v-if="!hideNext"
        class="btn-primary"
        @click="$emit('next')"
        :disabled="!wizard.canProceed"
      >
        {{ nextLabel }} →
      </button>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import StepIndicator from "./StepIndicator.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

withDefaults(defineProps<{
  showFooter?: boolean;
  hideNext?: boolean;
  nextLabel?: string;
}>(), {
  showFooter: true,
  hideNext: false,
  nextLabel: "下一步",
});

defineEmits<{ next: [] }>();

const wizard = useWizardStore();
const router = useRouter();
const { back, currentIndex, routeOrder } = useWizardNav();

const canGoBack = computed(() => {
  const idx = currentIndex();
  const name = router.currentRoute.value.name as string;
  return idx > 0 && name !== "deploy";
});

const visibleSteps = computed(() => [
  { label: "欢迎" },
  { label: "检查" },
  { label: "来源" },
  { label: "配置" },
  { label: "服务" },
  { label: "平台" },
  { label: "部署" },
  { label: "完成" },
]);
</script>

<style scoped>
.wizard-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  max-width: 720px;
  margin: 0 auto;
}

.wizard-header {
  padding: 16px 24px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  gap: 24px;
  background: var(--color-surface);
}

.brand { display: flex; align-items: center; gap: 8px; font-weight: 600; }
.brand-icon { font-size: 20px; }

.wizard-body {
  flex: 1;
  overflow-y: auto;
  padding: 32px 24px;
}

.wizard-footer {
  padding: 16px 24px;
  border-top: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  background: var(--color-surface);
}

.spacer { flex: 1; }
</style>
