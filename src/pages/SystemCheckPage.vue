<template>
  <WizardLayout next-label="下一步" @next="handleNext" :show-footer="true">
    <div class="system-check">
      <h2>系统环境检查</h2>
      <p class="desc">正在检查安装环境，必需项（标记为"必需"）全部通过后方可继续。</p>

      <div class="check-guide">
        <div class="guide-item"><span class="badge required">必需</span> 未通过则无法继续安装，需先解决</div>
        <div class="guide-item"><span class="badge optional">可选</span> 建议满足，不影响继续安装</div>
        <div class="guide-item">🔐 <strong>运行权限</strong>：以 root/管理员运行可注册系统级服务；普通用户运行注册用户级服务</div>
      </div>

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
import { computed, onMounted } from "vue";
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
  try {
    const results = await tauri.runSystemCheck();
    wizard.setChecks(results);
    wizard.setReady(allRequired.value);
  } catch {
    // 检查失败时仍允许继续（用户可手动判断环境）
    wizard.setReady(true);
  }
});

function handleNext() {
  if (allRequired.value) next();
}
</script>

<style scoped>
.system-check { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); }
.check-guide {
  background: #f8fafc; border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 12px 14px;
  display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: #475569;
}
.guide-item { display: flex; align-items: center; gap: 8px; }
.badge {
  padding: 1px 7px; border-radius: 10px; font-size: 11px; font-weight: 600; flex-shrink: 0;
}
.badge.required { background: #fee2e2; color: #991b1b; }
.badge.optional { background: #e0f2fe; color: #075985; }
.check-list { display: flex; flex-direction: column; gap: 8px; }
.loading { text-align: center; color: var(--color-muted); padding: 32px; }
.summary { font-size: 14px; font-weight: 600; }
.ok { color: var(--color-success); }
.err { color: var(--color-error); }
</style>
