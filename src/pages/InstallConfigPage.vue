<template>
  <WizardLayout @next="handleNext">
    <div class="install-config">
      <h2>安装路径</h2>

      <div class="field">
        <label>安装目录</label>
        <div class="input-row">
          <input type="text" v-model="config.installPath" @input="validate" />
        </div>
        <span class="hint">推荐路径：{{ defaultPath }}</span>
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
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();
const defaultPath = ref("/opt/openclaw");

onMounted(() => { validate(); });

function validate() {
  wizard.setReady(!!config.installPath.trim());
}

function handleNext() { next(); }
</script>

<style scoped>
.install-config { display: flex; flex-direction: column; gap: 20px; }
h2 { font-size: 20px; font-weight: 700; }
.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
.checkbox-row { display: flex; gap: 8px; align-items: center; cursor: pointer; }
.input-row { display: flex; gap: 8px; }
input[type="text"] {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 13px;
}
.hint { font-size: 12px; color: var(--color-muted); }
.hint-sm { font-size: 12px; color: var(--color-muted); margin-left: 24px; }
</style>
