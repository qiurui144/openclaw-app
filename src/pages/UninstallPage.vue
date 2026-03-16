<template>
  <WizardLayout :show-footer="false">
    <div class="uninstall-page">
      <h2>卸载 OpenClaw</h2>

      <!-- 安装信息 -->
      <div class="info-card">
        <div class="info-row">
          <span class="info-label">当前版本</span>
          <span class="info-value">{{ wizard.existingVersion }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">安装路径</span>
          <span class="info-value mono">{{ wizard.existingPath }}</span>
        </div>
      </div>

      <!-- 警告 -->
      <div class="warn-box" v-if="!done">
        <div class="warn-title">⚠️ 卸载将执行以下操作：</div>
        <ul class="warn-list">
          <li>停止并删除 OpenClaw 系统服务</li>
          <li>删除安装目录 <code>{{ wizard.existingPath }}</code></li>
          <li>删除配置目录 <code>~/.openclaw</code>（含 API Key、平台凭据）</li>
        </ul>
        <div class="warn-irrev">此操作不可撤销，配置数据将永久丢失。</div>
      </div>

      <!-- 确认输入 -->
      <div class="confirm-section" v-if="!running && !done">
        <label class="confirm-label">
          在下方输入 <code>uninstall</code> 以确认：
        </label>
        <input
          v-model="confirmText"
          type="text"
          class="confirm-input"
          placeholder="uninstall"
          @keyup.enter="confirmText === 'uninstall' && doUninstall()"
        />
      </div>

      <!-- 执行进度 -->
      <div class="progress-box" v-if="running && !steps.length">
        <div class="spinner"></div>
        <span>正在卸载，请稍候…</span>
      </div>

      <!-- 卸载步骤清单 -->
      <div class="steps-list" v-if="steps.length">
        <div v-for="(s, i) in steps" :key="i" class="step-row" :class="{ ok: s.success, fail: !s.success }">
          <span class="step-icon">{{ s.success ? "✅" : "❌" }}</span>
          <span class="step-name">{{ s.step }}</span>
          <span class="step-detail">{{ s.detail }}</span>
        </div>
      </div>

      <!-- 成功 -->
      <div class="success-box" v-if="done && !error">
        <span class="success-icon">✅</span>
        <div>
          <div class="success-title">卸载完成</div>
          <div class="success-desc">OpenClaw 已成功从系统中移除，可关闭此窗口。</div>
        </div>
      </div>

      <!-- 失败 -->
      <div class="error-box" v-if="error">
        <strong>卸载失败：</strong>{{ error }}
        <br><span class="err-hint">可手动执行 <code>{{ wizard.existingPath }}/uninstall.sh</code></span>
      </div>

      <!-- 操作按钮 -->
      <div class="action-row" v-if="!done">
        <button class="btn-secondary" @click="goBack" :disabled="running">← 返回</button>
        <div class="spacer" />
        <button
          class="btn-danger"
          @click="doUninstall"
          :disabled="confirmText !== 'uninstall' || running"
        >
          {{ running ? "卸载中…" : "确认卸载" }}
        </button>
      </div>
      <div class="action-row" v-else>
        <div class="spacer" />
        <button class="btn-secondary" @click="closeApp">关闭</button>
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
const { back } = useWizardNav();

interface UninstallStep { step: string; success: boolean; detail: string; }

const confirmText = ref("");
const running = ref(false);
const done = ref(false);
const error = ref<string | null>(null);
const steps = ref<UninstallStep[]>([]);

onMounted(() => { wizard.setReady(true); });

async function doUninstall() {
  if (!wizard.existingPath) return;
  running.value = true;
  error.value = null;
  steps.value = [];
  try {
    const results = await tauri.runUninstall(wizard.existingPath);
    steps.value = results;
    const allOk = results.every((s) => s.success);
    if (allOk) {
      done.value = true;
    } else {
      error.value = results.filter((s) => !s.success).map((s) => `${s.step}: ${s.detail}`).join("; ");
    }
  } catch (e: unknown) {
    error.value = String(e);
  } finally {
    running.value = false;
  }
}

function goBack() { back(); }

async function closeApp() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    getCurrentWindow().close();
  } catch {
    window.close();
  }
}
</script>

<style scoped>
.uninstall-page { display: flex; flex-direction: column; gap: 20px; }
h2 { font-size: 20px; font-weight: 700; }

.info-card {
  border: 1px solid var(--color-border); border-radius: var(--radius);
  padding: 12px 16px; background: var(--color-surface);
  display: flex; flex-direction: column; gap: 8px;
}
.info-row { display: flex; gap: 12px; align-items: baseline; }
.info-label { font-size: 12px; color: var(--color-muted); width: 72px; flex-shrink: 0; }
.info-value { font-size: 13px; font-weight: 500; }
.mono { font-family: monospace; }

.warn-box {
  background: #fef2f2; border: 1px solid #fecaca;
  border-radius: var(--radius); padding: 14px 16px;
}
.warn-title { font-weight: 600; font-size: 14px; color: #991b1b; margin-bottom: 8px; }
.warn-list {
  margin: 0; padding-left: 20px;
  display: flex; flex-direction: column; gap: 4px;
  font-size: 13px; color: #7f1d1d;
}
.warn-list li { line-height: 1.6; }
.warn-irrev { margin-top: 10px; font-size: 12px; font-weight: 600; color: #991b1b; }

.confirm-section { display: flex; flex-direction: column; gap: 8px; }
.confirm-label { font-size: 13px; color: var(--color-muted); }
.confirm-input {
  padding: 10px 14px; border: 2px solid var(--color-border);
  border-radius: var(--radius); font-size: 14px; font-family: monospace;
  transition: border-color .15s;
}
.confirm-input:focus { outline: none; border-color: var(--color-error, #ef4444); }

.progress-box {
  display: flex; align-items: center; gap: 12px;
  font-size: 14px; color: var(--color-muted);
}
.spinner {
  width: 20px; height: 20px; border-radius: 50%;
  border: 3px solid var(--color-border);
  border-top-color: var(--color-primary);
  animation: spin 0.8s linear infinite; flex-shrink: 0;
}
@keyframes spin { to { transform: rotate(360deg); } }

.success-box {
  display: flex; align-items: center; gap: 14px;
  background: #f0fdf4; border: 1px solid #bbf7d0;
  border-radius: var(--radius); padding: 16px;
}
.success-icon { font-size: 28px; flex-shrink: 0; }
.success-title { font-weight: 600; font-size: 15px; }
.success-desc { font-size: 13px; color: var(--color-muted); margin-top: 3px; }

.error-box {
  background: #fef2f2; border: 1px solid #fecaca;
  border-radius: var(--radius); padding: 12px 14px;
  font-size: 13px; color: #991b1b;
}
.err-hint { font-size: 12px; color: var(--color-muted); }

.steps-list {
  display: flex; flex-direction: column; gap: 8px;
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 14px 16px;
}
.step-row { display: flex; align-items: center; gap: 10px; font-size: 13px; }
.step-icon { flex-shrink: 0; }
.step-name { font-weight: 500; min-width: 100px; }
.step-detail { color: var(--color-muted); flex: 1; }
.step-row.fail .step-detail { color: var(--color-error); }

.action-row { display: flex; align-items: center; gap: 10px; margin-top: 4px; }
.spacer { flex: 1; }

.btn-danger {
  padding: 9px 20px; background: #ef4444; color: #fff;
  border: none; border-radius: var(--radius);
  font-size: 14px; font-weight: 600; cursor: pointer;
  transition: opacity .15s;
}
.btn-danger:hover:not(:disabled) { opacity: 0.85; }
.btn-danger:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
