<template>
  <WizardLayout next-label="下一步" @next="handleNext" :show-footer="true">
    <div class="system-check">
      <h2>系统环境检查</h2>
      <p class="desc">正在检查安装环境，必需项全部通过后方可继续。</p>

      <div class="check-list" v-if="checks.length">
        <CheckItemComponent v-for="c in checks" :key="c.id" :item="c" />
      </div>
      <div v-else class="loading">
        <span>⏳ 正在检查中…</span>
      </div>

      <div class="summary" v-if="checks.length">
        <span v-if="allRequired" class="ok">✓ 所有必需项已通过</span>
        <span v-else class="err">✕ {{ failedRequired.length }} 项必需检查未通过</span>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import CheckItemComponent from "@/components/CheckItem.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const wizard = useWizardStore();
const { next } = useWizardNav();
const checks = computed(() => wizard.systemChecks);

const failedRequired = computed(() =>
  checks.value.filter((c) => c.required && c.status === "error")
);
const allRequired = computed(() => failedRequired.value.length === 0 && checks.value.length > 0);

onMounted(async () => {
  wizard.setReady(false);
  const results = await tauri.runSystemCheck();
  wizard.setChecks(results);
  wizard.setReady(allRequired.value);
});

function handleNext() {
  if (allRequired.value) next();
}
</script>

<style scoped>
.system-check { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); }
.check-list { display: flex; flex-direction: column; gap: 8px; }
.loading { text-align: center; color: var(--color-muted); padding: 32px; }
.summary { font-size: 14px; font-weight: 600; }
.ok { color: var(--color-success); }
.err { color: var(--color-error); }
</style>
