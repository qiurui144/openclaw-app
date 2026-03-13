<template>
  <WizardLayout next-label="开始安装" @next="handleNext" :show-footer="true" :hide-next="false">
    <div class="welcome">
      <div class="hero">
        <div class="logo">🦾</div>
        <h1>OpenClaw</h1>
        <p class="subtitle">企业级多平台机器人网关 · 一键部署</p>
        <p class="version">向导版本 {{ wizardVersion }}</p>
      </div>

      <div class="existing-banner" v-if="wizard.isExistingInstall">
        <span>⚠️ 检测到已安装版本 {{ wizard.existingVersion }}（{{ wizard.existingPath }}）</span>
        <div class="existing-actions">
          <button class="btn-secondary" @click="mode = 'upgrade'">升级安装</button>
          <button class="btn-secondary" style="color:var(--color-error)" @click="mode = 'fresh'">全新安装</button>
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
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const wizard = useWizardStore();
const { next } = useWizardNav();
const wizardVersion = typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "1.0.0";
const mode = ref<"fresh" | "upgrade">("fresh");

const features = [
  { icon: "📦", title: "全量内置", desc: "内嵌 Node.js，无需联网即可完成安装" },
  { icon: "🔌", title: "平台集成", desc: "飞书 / 企业微信 / 钉钉 / QQ Work" },
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

async function handleNext() {
  next();
}
</script>

<style scoped>
.welcome { display: flex; flex-direction: column; gap: 24px; }
.hero { text-align: center; padding: 16px 0; }
.logo { font-size: 48px; margin-bottom: 8px; }
h1 { font-size: 28px; font-weight: 700; }
.subtitle { color: var(--color-muted); margin-top: 4px; }
.version { font-size: 12px; color: var(--color-border); margin-top: 8px; }
.existing-banner {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 12px 16px;
  display: flex; align-items: center; justify-content: space-between;
}
.existing-actions { display: flex; gap: 8px; }
.feature-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.feature-card {
  display: flex; gap: 12px; align-items: flex-start;
  padding: 16px; background: var(--color-surface);
  border: 1px solid var(--color-border); border-radius: var(--radius);
  box-shadow: var(--shadow);
}
.f-icon { font-size: 24px; flex-shrink: 0; }
.f-title { font-weight: 600; }
.f-desc { font-size: 12px; color: var(--color-muted); margin-top: 2px; }
</style>
