<template>
  <WizardLayout :show-footer="false">
    <div class="welcome">
      <div class="hero">
        <div class="logo">🦾</div>
        <h1>OpenClaw</h1>
        <p class="subtitle">企业级多平台机器人网关</p>
        <p class="version">向导 {{ wizardVersion }}</p>
      </div>

      <!-- 检测到已安装：显示三个操作卡片 -->
      <template v-if="wizard.isExistingInstall">
        <div class="existing-info">
          <span class="existing-icon">✅</span>
          <div>
            <div class="existing-title">已安装 v{{ wizard.existingVersion }}</div>
            <div class="existing-path">{{ wizard.existingPath }}</div>
          </div>
        </div>

        <div class="mode-grid">
          <button class="mode-card" @click="pickMode('install')">
            <span class="mode-icon">🔄</span>
            <div class="mode-name">重新安装</div>
            <div class="mode-desc">全新部署，覆盖当前安装，重新配置所有选项</div>
          </button>
          <button class="mode-card mode-card--primary" @click="pickMode('update')">
            <span class="mode-icon">⬆️</span>
            <div class="mode-name">更新</div>
            <div class="mode-desc">检查并升级 OpenClaw 至最新版本，保留现有配置</div>
          </button>
          <button class="mode-card mode-card--danger" @click="pickMode('uninstall')">
            <span class="mode-icon">🗑️</span>
            <div class="mode-name">卸载</div>
            <div class="mode-desc">停止服务并删除 OpenClaw 及所有相关文件</div>
          </button>
        </div>
      </template>

      <!-- 未安装：直接显示安装引导 -->
      <template v-else>
        <div class="steps-overview">
          <div class="steps-title">安装步骤（约 5-10 分钟）</div>
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

        <div class="feature-grid">
          <div class="feature-card" v-for="f in features" :key="f.icon">
            <span class="f-icon">{{ f.icon }}</span>
            <div>
              <div class="f-title">{{ f.title }}</div>
              <div class="f-desc">{{ f.desc }}</div>
            </div>
          </div>
        </div>

        <button class="btn-install" @click="pickMode('install')">
          开始安装 →
        </button>
      </template>
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

const features = [
  { icon: "📦", title: "全量内置", desc: "内嵌 Node.js，无需联网即可完成安装" },
  { icon: "🔌", title: "多平台接入", desc: "飞书 / 企业微信 / 钉钉 / QQ 机器人" },
  { icon: "🚀", title: "开机自启", desc: "注册系统服务，故障自动重启" },
  { icon: "🌐", title: "网页控制台", desc: "浏览器管理机器人、权限与日志" },
];

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
.welcome { display: flex; flex-direction: column; gap: 24px; }

.hero { text-align: center; padding: 12px 0 4px; }
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
.mode-card:hover {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px #dbeafe;
}
.mode-card--primary {
  border-color: var(--color-primary);
  background: #eff6ff;
}
.mode-card--primary:hover { box-shadow: 0 0 0 3px #bfdbfe; }
.mode-card--danger:hover {
  border-color: var(--color-error);
  box-shadow: 0 0 0 3px #fee2e2;
}
.mode-icon { font-size: 28px; }
.mode-name { font-weight: 700; font-size: 15px; }
.mode-desc { font-size: 12px; color: var(--color-muted); line-height: 1.5; }

/* 步骤概览（新安装） */
.steps-overview {
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

.feature-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.feature-card {
  display: flex; gap: 12px; align-items: flex-start;
  padding: 14px; background: var(--color-surface);
  border: 1px solid var(--color-border); border-radius: var(--radius);
}
.f-icon { font-size: 22px; flex-shrink: 0; }
.f-title { font-weight: 600; font-size: 13px; }
.f-desc { font-size: 12px; color: var(--color-muted); margin-top: 2px; }

.btn-install {
  width: 100%; padding: 14px;
  background: var(--color-primary); color: #fff;
  border: none; border-radius: var(--radius);
  font-size: 16px; font-weight: 600; cursor: pointer;
  transition: opacity .15s;
}
.btn-install:hover { opacity: 0.9; }
</style>
