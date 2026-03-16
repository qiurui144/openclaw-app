<template>
  <WizardLayout @next="handleNext">
    <div class="install-config">
      <h2>安装路径</h2>
      <p class="step-intro">选择 OpenClaw 的安装目录和系统服务方式。</p>

      <StepHelp>
        <ul>
          <li><strong>安装目录</strong>：OpenClaw 程序文件存放位置。留空将使用推荐路径。</li>
          <li><strong>root/管理员运行</strong>：默认安装到 <code>/opt/openclaw</code>（Linux）或 <code>C:\Program Files\openclaw</code>（Windows），注册<strong>系统级服务</strong>，开机自动运行无需用户登录。</li>
          <li><strong>普通用户运行</strong>：默认安装到 <code>~/openclaw</code>（Linux）或 <code>%LOCALAPPDATA%\openclaw</code>（Windows），注册<strong>用户级服务</strong>，需要用户登录后才会自启。</li>
          <li><strong>注册系统服务</strong>（推荐）：将 OpenClaw 注册为 systemd/计划任务服务，进程崩溃后自动重启。</li>
        </ul>
        <div class="tip">💡 服务器部署建议以 root/管理员身份运行向导，确保开机免登录自启。</div>
      </StepHelp>

      <div class="field">
        <label>安装目录</label>
        <div class="input-row">
          <input type="text" v-model="config.installPath" :placeholder="defaultPath" @input="validate" />
          <button class="btn-secondary" @click="validatePath" :disabled="validating">{{ validating ? "检测中…" : "检测" }}</button>
        </div>
        <span class="hint" v-if="!pathError">推荐路径：{{ defaultPath }}</span>
        <span class="hint path-error" v-if="pathError">{{ pathError }}</span>
        <span class="hint path-ok" v-if="pathOk">路径可用，磁盘空间充足</span>
      </div>

      <div class="field">
        <label class="checkbox-row">
          <input type="checkbox" v-model="config.installService" />
          注册系统服务（推荐）
        </label>
        <p class="hint-sm">将 OpenClaw 注册为系统服务，故障自动重启。</p>
      </div>

      <div class="field" v-if="config.installService">
        <label class="checkbox-row">
          <input type="checkbox" v-model="config.startOnBoot" />
          开机自动启动
        </label>
        <p class="hint-sm">系统启动时自动运行 OpenClaw 服务。</p>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import StepHelp from "@/components/StepHelp.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();
const defaultPath = ref("");
const validating = ref(false);
const pathError = ref("");
const pathOk = ref(false);

onMounted(async () => {
  try {
    const p = await tauri.getDefaultInstallPath();
    defaultPath.value = p;
    if (!config.installPath) config.installPath = p;
  } catch { /* ignore */ }
  validate();
});

function validate() {
  pathError.value = "";
  pathOk.value = false;
  wizard.setReady(!!config.installPath.trim());
}

async function validatePath() {
  const p = config.installPath.trim();
  if (!p) { pathError.value = "路径不能为空"; return; }
  validating.value = true;
  pathError.value = "";
  pathOk.value = false;
  try {
    await tauri.validateInstallPath(p);
    pathOk.value = true;
  } catch (e) {
    pathError.value = String(e);
    pathOk.value = false;
  } finally {
    validating.value = false;
  }
}

function handleNext() { next(); }
</script>

<style scoped>
.install-config { display: flex; flex-direction: column; gap: 20px; }
h2 { font-size: 20px; font-weight: 700; }
.step-intro { font-size: 13px; color: var(--color-muted); margin-top: -12px; }
.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
.checkbox-row { display: flex; gap: 8px; align-items: center; cursor: pointer; }
.input-row { display: flex; gap: 8px; }
input[type="text"] {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 13px;
}
.hint { font-size: 12px; color: var(--color-muted); }
.path-error { color: var(--color-error); font-weight: 500; }
.path-ok { color: var(--color-success); font-weight: 500; }
.hint-sm { font-size: 12px; color: var(--color-muted); margin-left: 24px; }
</style>
