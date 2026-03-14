<template>
  <WizardLayout :show-footer="false">
    <div class="update-page">
      <h2>更新 OpenClaw</h2>

      <!-- 当前版本信息 -->
      <div class="version-row">
        <div class="version-card">
          <div class="vc-label">当前版本</div>
          <div class="vc-value">{{ wizard.existingVersion ?? "未知" }}</div>
          <div class="vc-path">{{ wizard.existingPath }}</div>
        </div>
        <div class="arrow">→</div>
        <div class="version-card" :class="{ 'version-card--new': hasUpdate }">
          <div class="vc-label">最新版本</div>
          <div class="vc-value">
            <template v-if="checking">检查中…</template>
            <template v-else-if="updateInfo">{{ updateInfo.version }}</template>
            <template v-else-if="checkError">获取失败</template>
            <template v-else>{{ wizard.existingVersion }}</template>
          </div>
          <div class="vc-path" v-if="updateInfo && hasUpdate">有新版本可用</div>
          <div class="vc-path" v-else-if="!checking && !checkError">已是最新版本</div>
        </div>
      </div>

      <!-- 更新日志 -->
      <div class="release-notes" v-if="updateInfo?.release_notes">
        <div class="rn-label">更新内容</div>
        <div class="rn-body">{{ updateInfo.release_notes }}</div>
      </div>

      <!-- 错误提示 -->
      <div class="error-box" v-if="checkError">
        <strong>检查更新失败：</strong>{{ checkError }}
        <br><span class="err-hint">请检查网络连接后重试</span>
      </div>

      <!-- 更新进度 -->
      <div class="progress-box" v-if="applying">
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: applyPercent + '%' }"></div>
        </div>
        <div class="progress-msg">{{ applyMsg }}</div>
      </div>

      <!-- 成功 -->
      <div class="success-box" v-if="done">
        <span class="success-icon">✅</span>
        <div>
          <div class="success-title">更新完成</div>
          <div class="success-desc">OpenClaw 已升级至 v{{ updateInfo?.version }}，服务已自动重启</div>
        </div>
      </div>

      <!-- 操作按钮 -->
      <div class="action-row" v-if="!done">
        <button class="btn-secondary" @click="goBack" :disabled="applying">← 返回</button>
        <div class="spacer" />
        <button class="btn-secondary" @click="checkForUpdate" :disabled="checking || applying">
          {{ checking ? "检查中…" : "重新检查" }}
        </button>
        <button
          class="btn-primary"
          @click="applyUpdate"
          :disabled="!hasUpdate || applying || checking"
        >
          {{ applying ? "更新中…" : "立即更新" }} →
        </button>
      </div>
      <div class="action-row" v-else>
        <div class="spacer" />
        <button class="btn-primary" @click="finish">完成 →</button>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri, type UpdateInfo } from "@/composables/useTauri";

const wizard = useWizardStore();
const { next, back } = useWizardNav();

const checking = ref(false);
const applying = ref(false);
const done = ref(false);
const checkError = ref<string | null>(null);
const updateInfo = ref<UpdateInfo | null>(null);
const applyPercent = ref(0);
const applyMsg = ref("");

const hasUpdate = computed(() => {
  if (!updateInfo.value || !wizard.existingVersion) return false;
  return updateInfo.value.version !== wizard.existingVersion;
});

onMounted(() => {
  wizard.setReady(true);
  checkForUpdate();
});

async function checkForUpdate() {
  checking.value = true;
  checkError.value = null;
  updateInfo.value = null;
  try {
    updateInfo.value = await tauri.checkUpdate();
  } catch (e: unknown) {
    checkError.value = String(e);
  } finally {
    checking.value = false;
  }
}

async function applyUpdate() {
  if (!updateInfo.value || !wizard.existingPath) return;
  applying.value = true;
  applyPercent.value = 10;
  applyMsg.value = "下载更新包…";
  try {
    applyPercent.value = 30;
    applyMsg.value = "下载中，请稍候…";
    await tauri.applyUpdate(
      wizard.existingPath,
      updateInfo.value.download_url,
      updateInfo.value.sha256,
    );
    applyPercent.value = 100;
    done.value = true;
  } catch (e: unknown) {
    checkError.value = `更新失败：${e}`;
  } finally {
    applying.value = false;
  }
}

function goBack() { back(); }
function finish() { next(); }
</script>

<style scoped>
.update-page { display: flex; flex-direction: column; gap: 20px; }
h2 { font-size: 20px; font-weight: 700; }

.version-row {
  display: flex; align-items: center; gap: 16px;
}
.version-card {
  flex: 1; border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 14px 16px;
  background: var(--color-surface);
}
.version-card--new { border-color: var(--color-primary); background: #eff6ff; }
.vc-label { font-size: 11px; color: var(--color-muted); text-transform: uppercase; letter-spacing: .5px; }
.vc-value { font-size: 20px; font-weight: 700; margin: 4px 0; font-family: monospace; }
.vc-path { font-size: 11px; color: var(--color-muted); }
.arrow { font-size: 24px; color: var(--color-muted); flex-shrink: 0; }

.release-notes {
  border: 1px solid var(--color-border); border-radius: var(--radius);
  padding: 12px 14px; background: var(--color-surface);
}
.rn-label { font-size: 11px; color: var(--color-muted); text-transform: uppercase; letter-spacing: .5px; margin-bottom: 6px; }
.rn-body { font-size: 13px; white-space: pre-wrap; line-height: 1.6; color: #334155; }

.error-box {
  background: #fef2f2; border: 1px solid #fecaca;
  border-radius: var(--radius); padding: 12px 14px;
  font-size: 13px; color: #991b1b;
}
.err-hint { font-size: 12px; color: var(--color-muted); }

.progress-box { display: flex; flex-direction: column; gap: 8px; }
.progress-bar { height: 6px; background: var(--color-border); border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: var(--color-primary); border-radius: 3px; transition: width .3s; }
.progress-msg { font-size: 12px; color: var(--color-muted); }

.success-box {
  display: flex; align-items: center; gap: 14px;
  background: #f0fdf4; border: 1px solid #bbf7d0;
  border-radius: var(--radius); padding: 16px;
}
.success-icon { font-size: 28px; flex-shrink: 0; }
.success-title { font-weight: 600; font-size: 15px; }
.success-desc { font-size: 13px; color: var(--color-muted); margin-top: 3px; }

.action-row { display: flex; align-items: center; gap: 10px; margin-top: 4px; }
.spacer { flex: 1; }
</style>
