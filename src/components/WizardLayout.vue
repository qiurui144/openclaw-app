<template>
  <div class="wizard-shell">
    <header class="wizard-header">
      <div class="brand">
        <span class="brand-icon">🦾</span>
        <span class="brand-name">OpenClaw 部署向导</span>
      </div>
      <StepIndicator :steps="visibleSteps" :current="currentIndex()" />
      <button class="help-btn" @click="showManual = true" title="操作手册">?</button>
    </header>
    <ManualModal v-model="showManual" />

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
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import StepIndicator from "./StepIndicator.vue";
import ManualModal from "./ManualModal.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const showManual = ref(false);

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
const { back, currentIndex } = useWizardNav();

const canGoBack = computed(() => {
  const idx = currentIndex();
  const name = router.currentRoute.value.name as string;
  return idx > 0 && name !== "deploy";
});

const STEPS_BY_MODE: Record<string, { label: string }[]> = {
  install: [
    { label: "欢迎" }, { label: "检查" }, { label: "来源" },
    { label: "配置" }, { label: "服务" }, { label: "AI" },
    { label: "平台" }, { label: "部署" }, { label: "完成" },
  ],
  update: [{ label: "欢迎" }, { label: "更新" }, { label: "完成" }],
  uninstall: [{ label: "欢迎" }, { label: "卸载" }],
};

const visibleSteps = computed(() => STEPS_BY_MODE[wizard.wizardMode] ?? STEPS_BY_MODE.install);
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
  padding: 12px 24px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  gap: 24px;
  background: var(--color-surface);
}

.help-btn {
  margin-left: auto; flex-shrink: 0;
  width: 28px; height: 28px; border-radius: 50%;
  border: 1px solid var(--color-border); background: none;
  font-size: 14px; font-weight: 700; color: var(--color-muted);
  cursor: pointer; display: flex; align-items: center; justify-content: center;
}
.help-btn:hover { border-color: var(--color-primary); color: var(--color-primary); }

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
