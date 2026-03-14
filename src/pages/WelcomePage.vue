<template>
  <WizardLayout :show-footer="false">
    <div class="welcome">
      <div class="hero">
        <div class="logo">🦾</div>
        <h1>OpenClaw</h1>
        <p class="subtitle">企业级多平台机器人网关</p>
        <p class="version">向导 {{ wizardVersion }}</p>
      </div>

      <!-- 已检测到安装时显示版本信息 -->
      <div v-if="wizard.isExistingInstall" class="existing-info">
        <span class="existing-icon">✅</span>
        <div>
          <div class="existing-title">已安装 v{{ wizard.existingVersion }}</div>
          <div class="existing-path">{{ wizard.existingPath }}</div>
        </div>
      </div>

      <!-- 始终显示三个操作入口 -->
      <div class="mode-grid">
        <button class="mode-card" @click="pickMode('install')">
          <span class="mode-icon">📦</span>
          <div class="mode-name">安装</div>
          <div class="mode-desc">全新部署，配置路径、服务、平台集成后一键安装</div>
        </button>

        <button
          class="mode-card mode-card--primary"
          :class="{ 'mode-card--disabled': !wizard.isExistingInstall }"
          @click="pickMode('update')"
        >
          <span class="mode-icon">⬆️</span>
          <div class="mode-name">更新</div>
          <div class="mode-desc">
            <template v-if="wizard.isExistingInstall">升级至最新版本，保留现有配置</template>
            <template v-else>未检测到已有安装</template>
          </div>
        </button>

        <button
          class="mode-card mode-card--danger"
          :class="{ 'mode-card--disabled': !wizard.isExistingInstall }"
          @click="pickMode('uninstall')"
        >
          <span class="mode-icon">🗑️</span>
          <div class="mode-name">卸载</div>
          <div class="mode-desc">
            <template v-if="wizard.isExistingInstall">停止服务并删除 OpenClaw 及相关文件</template>
            <template v-else>未检测到已有安装</template>
          </div>
        </button>
      </div>

      <!-- 首次安装时的功能介绍（折叠展示，不遮挡主操作） -->
      <div v-if="!wizard.isExistingInstall" class="install-hint">
        <div class="steps-title">安装流程（约 5-10 分钟）</div>
        <div class="steps-row">
          <span class="step-pill">1 环境检查</span>
          <span class="step-sep">→</span>
          <span class="step-pill">2 来源</span>
          <span class="step-sep">→</span>
          <span class="step-pill">3 路径配置</span>
          <span class="step-sep">→</span>
          <span class="step-pill">4 服务设置</span>
          <span class="step-sep">→</span>
          <span class="step-pill">5 平台集成</span>
          <span class="step-sep">→</span>
          <span class="step-pill">6 一键部署</span>
        </div>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore, type WizardMode } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const wizard = useWizardStore();
const { next } = useWizardNav();
const wizardVersion = typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "1.0.0";

onMounted(async () => {
  wizard.setReady(true);
  try {
    const meta = await tauri.readDeployMeta();
    if (meta) {
      wizard.setExistingInstall(meta.version, meta.install_path);
    }
  } catch { /* 无已有安装 */ }
});

function pickMode(mode: WizardMode) {
  wizard.setWizardMode(mode);
  next();
}
</script>

<style scoped>
.welcome { display: flex; flex-direction: column; gap: 20px; }

.hero { text-align: center; padding: 8px 0 0; }
.logo { font-size: 44px; margin-bottom: 6px; }
h1 { font-size: 28px; font-weight: 700; }
.subtitle { color: var(--color-muted); margin-top: 4px; font-size: 14px; }
.version { font-size: 11px; color: var(--color-border); margin-top: 6px; }

/* 已安装信息条 */
.existing-info {
  display: flex; align-items: center; gap: 12px;
  background: #f0fdf4; border: 1px solid #bbf7d0;
  border-radius: var(--radius); padding: 12px 16px;
}
.existing-icon { font-size: 20px; flex-shrink: 0; }
.existing-title { font-weight: 600; font-size: 14px; }
.existing-path { font-size: 12px; color: var(--color-muted); margin-top: 2px; font-family: monospace; }

/* 模式选择卡片 */
.mode-grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }

.mode-card {
  display: flex; flex-direction: column; align-items: center; gap: 8px;
  padding: 20px 14px; text-align: center;
  border: 2px solid var(--color-border); border-radius: var(--radius);
  background: var(--color-surface); cursor: pointer;
  transition: border-color .15s, box-shadow .15s;
}
.mode-card:not(.mode-card--disabled):hover {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px #dbeafe;
}
.mode-card--primary {
  border-color: var(--color-primary);
  background: #eff6ff;
}
.mode-card--primary:not(.mode-card--disabled):hover { box-shadow: 0 0 0 3px #bfdbfe; }
.mode-card--danger:not(.mode-card--disabled):hover {
  border-color: var(--color-error);
  box-shadow: 0 0 0 3px #fee2e2;
}
.mode-card--disabled {
  opacity: 0.45; cursor: not-allowed; border-color: var(--color-border);
  background: var(--color-surface);
}
.mode-icon { font-size: 28px; }
.mode-name { font-weight: 700; font-size: 15px; }
.mode-desc { font-size: 12px; color: var(--color-muted); line-height: 1.5; }

/* 安装步骤提示 */
.install-hint {
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 12px 16px;
}
.steps-title { font-size: 12px; color: var(--color-muted); margin-bottom: 8px; }
.steps-row { display: flex; align-items: center; flex-wrap: wrap; gap: 4px; }
.step-pill {
  background: #eff6ff; color: var(--color-primary); border-radius: 12px;
  padding: 3px 10px; font-size: 12px; font-weight: 500; white-space: nowrap;
}
.step-sep { color: var(--color-border); font-size: 12px; }
</style>
