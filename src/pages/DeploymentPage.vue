<template>
  <WizardLayout :show-footer="false">
    <div class="deploy-page">
      <h2>正在部署</h2>

      <div class="status-row">
        <span class="status-icon" :class="wizard.deployStatus">{{ statusIcon }}</span>
        <span class="status-msg">{{ wizard.deployProgress.message || "准备中…" }}</span>
      </div>

      <div class="progress-track">
        <div
          class="progress-fill"
          :class="{ error: wizard.deployStatus === 'failed' }"
          :style="{ width: wizard.deployProgress.percent + '%' }"
        />
      </div>
      <div class="progress-label">{{ wizard.deployProgress.percent }}%</div>

      <LogPanel :logs="wizard.deployLogs" />

      <div class="fail-actions" v-if="wizard.deployStatus === 'failed'">
        <p class="err-msg">{{ errorReason }}</p>
        <button class="btn-secondary" @click="goTo('install')">返回重新配置</button>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import LogPanel from "@/components/LogPanel.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { useWizardNav } from "@/composables/useWizardNav";
import { useDeployEvents, tauri } from "@/composables/useTauri";

const wizard = useWizardStore();
const config = useConfigStore();
const { goTo } = useWizardNav();
const errorReason = ref("");

const statusIcon = computed(() => ({
  idle: "⏳", running: "⚡", done: "✅", failed: "❌",
}[wizard.deployStatus]));

const { subscribe } = useDeployEvents(
  (p) => {
    wizard.updateProgress(p);
    wizard.setDeployStatus("running");
  },
  () => {
    wizard.setDeployStatus("done");
    tauri.clashStop().catch(() => {});
    goTo("finish");
  },
  (reason) => {
    wizard.setDeployStatus("failed");
    errorReason.value = reason;
    tauri.clashStop().catch(() => {});
  },
);

onMounted(async () => {
  await subscribe();
  wizard.setDeployStatus("running");
  const dto = {
    ...config.toDto(),
    source_mode: buildSourceMode(),
  };
  await tauri.startDeploy(dto);
});

function buildSourceMode() {
  const m = wizard.sourceMode;
  if (m === "bundled") return { type: "bundled" };
  if (m === "local_zip") return { type: "local_zip", zip_path: config.localZipPath };
  return {
    type: "online",
    proxy_url: wizard.clashAccepted && config.clashSubscriptionUrl
      ? config.clashSubscriptionUrl : null,
  };
}
</script>

<style scoped>
.deploy-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.status-row { display: flex; align-items: center; gap: 12px; }
.status-icon { font-size: 24px; }
.progress-track { height: 8px; background: var(--color-border); border-radius: 4px; overflow: hidden; }
.progress-fill { height: 100%; background: var(--color-primary); transition: width .4s ease; border-radius: 4px; }
.progress-fill.error { background: var(--color-error); }
.progress-label { font-size: 12px; color: var(--color-muted); text-align: right; }
.fail-actions { display: flex; flex-direction: column; gap: 8px; }
.err-msg { color: var(--color-error); font-size: 13px; }
</style>
