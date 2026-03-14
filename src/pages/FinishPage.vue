<template>
  <WizardLayout :show-footer="false">
    <div class="finish-page">
      <div class="warn-banner" v-if="wizard.deployStatus !== 'done'">
        ⚠️ 服务尚未运行，请查看日志或手动启动
      </div>

      <div class="hero">
        <div class="success-icon">✅</div>
        <h2>部署完成！</h2>
        <p class="subtitle">OpenClaw 已成功安装到您的系统。</p>
      </div>

      <div class="summary-card">
        <div class="summary-row">
          <span class="s-label">服务地址</span>
          <span class="s-val">http://127.0.0.1:{{ config.servicePort }}</span>
          <button class="copy-btn" @click="copyText('http://127.0.0.1:' + config.servicePort)">复制</button>
        </div>
        <div class="summary-row">
          <span class="s-label">配置文件</span>
          <span class="s-val">~/.openclaw/openclaw.json</span>
        </div>
        <div class="summary-row">
          <span class="s-label">安装路径</span>
          <span class="s-val">{{ config.installPath }}</span>
        </div>
      </div>

      <div class="action-row">
        <button class="btn-primary" @click="openConsole">🌐 打开管理控制台</button>
      </div>

      <div class="next-steps">
        <div class="ns-title">接下来您可以：</div>
        <div class="ns-item">🔐 <span>用设置的管理员密码登录控制台</span></div>
        <div class="ns-item">🔌 <span>在控制台「平台管理」中添加企业微信/钉钉/飞书机器人</span></div>
        <div class="ns-item">🧩 <span>在「Skills 管理」中安装所需的 AI 技能插件</span></div>
        <div class="ns-item">📋 <span>在「日志」中监控机器人运行状态</span></div>
        <div class="ns-item">🔄 <span>通过控制台「系统设置」检查版本更新</span></div>
      </div>

      <div class="skills-section" v-if="updatableSkills.length">
        <h3>可更新的 Skills（{{ updatableSkills.length }}）</h3>
        <div class="skill-list">
          <div class="skill-row" v-for="s in updatableSkills" :key="s.name">
            <span>{{ s.name }}</span>
            <span class="version">{{ s.current_version }} → {{ s.latest_version }}</span>
          </div>
        </div>
        <button class="btn-primary" @click="updateAll" :disabled="updating">
          {{ updating ? "更新中…" : "全部更新" }}
        </button>
      </div>

      <div class="feedback">
        <a href="https://github.com/openclaw/openclaw/issues" target="_blank">🐛 反馈问题</a>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { tauri, type SkillInfo } from "@/composables/useTauri";

const wizard = useWizardStore();
const config = useConfigStore();
const updatableSkills = ref<SkillInfo[]>([]);
const updating = ref(false);

onMounted(async () => {
  try {
    const skills = await tauri.listSkills(config.installPath);
    updatableSkills.value = skills.filter((s) => s.update_available);
  } catch { /* 忽略 */ }
});

function openConsole() {
  tauri.openUrl(`http://127.0.0.1:${config.servicePort}`);
}

async function updateAll() {
  updating.value = true;
  try {
    await tauri.updateSkills(
      config.installPath,
      updatableSkills.value.map((s) => s.name),
      wizard.clashAccepted ? config.clashSubscriptionUrl : undefined,
    );
    updatableSkills.value = [];
  } finally {
    updating.value = false;
  }
}

function copyText(text: string) {
  navigator.clipboard.writeText(text).catch(() => {});
}
</script>

<style scoped>
.finish-page { display: flex; flex-direction: column; gap: 20px; }
.warn-banner {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 12px 16px;
  font-size: 13px; color: #92400e;
}
.hero { text-align: center; padding: 8px 0; }
.success-icon { font-size: 48px; }
h2 { font-size: 24px; font-weight: 700; margin-top: 8px; }
.subtitle { color: var(--color-muted); margin-top: 4px; }
.summary-card {
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 16px;
  display: flex; flex-direction: column; gap: 10px;
}
.summary-row { display: flex; align-items: center; gap: 8px; font-size: 13px; }
.s-label { color: var(--color-muted); min-width: 80px; }
.s-val { font-family: monospace; flex: 1; }
.copy-btn { padding: 2px 8px; font-size: 11px; background: var(--color-bg); border: 1px solid var(--color-border); border-radius: 4px; }
.action-row { display: flex; justify-content: center; }
.btn-primary { padding: 10px 24px; font-size: 15px; }
.skills-section h3 { font-size: 15px; font-weight: 600; margin-bottom: 10px; }
.skill-list { display: flex; flex-direction: column; gap: 6px; margin-bottom: 10px; }
.skill-row { display: flex; justify-content: space-between; font-size: 13px; }
.version { color: var(--color-muted); }
.next-steps {
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 14px 16px;
  display: flex; flex-direction: column; gap: 8px;
}
.ns-title { font-size: 13px; font-weight: 600; margin-bottom: 4px; }
.ns-item { display: flex; align-items: flex-start; gap: 8px; font-size: 13px; color: #475569; }
.feedback { text-align: center; font-size: 13px; }
.feedback a { color: var(--color-muted); }
</style>
