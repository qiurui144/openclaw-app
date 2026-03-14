<template>
  <WizardLayout @next="handleNext">
    <div class="source-page">
      <h2>选择安装来源</h2>
      <p class="desc">根据您的网络环境选择合适的安装方式。Full Bundle 版（离线包）最稳定，推荐优先使用。</p>

      <div class="options">
        <label class="option" :class="{ selected: selected === 'bundled', disabled: !hasBundled }">
          <input type="radio" value="bundled" v-model="selected" :disabled="!hasBundled" />
          <span class="opt-icon">📦</span>
          <div>
            <div class="opt-title">使用内置离线包（推荐）<span v-if="!hasBundled" class="unavail">此版本不含，请下载 Full Bundle</span></div>
            <div class="opt-desc">无需网络，离线直接安装，最稳定。适合内网环境或网络不好的情况。</div>
          </div>
        </label>

        <label class="option" :class="{ selected: selected === 'online' }">
          <input type="radio" value="online" v-model="selected" />
          <span class="opt-icon">🌐</span>
          <div>
            <div class="opt-title">从网络下载最新版本</div>
            <div class="opt-desc">需要访问 npm/GitHub，支持配置 Clash 代理。适合网络畅通的环境。</div>
          </div>
        </label>

        <label class="option" :class="{ selected: selected === 'local_zip' }">
          <input type="radio" value="local_zip" v-model="selected" />
          <span class="opt-icon">📁</span>
          <div>
            <div class="opt-title">导入本地安装包</div>
            <div class="opt-desc">使用已手动下载的 openclaw.tgz 压缩包。适合在有网络的机器上提前下载，再拷贝到内网安装。</div>
          </div>
        </label>
      </div>

      <div class="zip-zone" v-if="selected === 'local_zip'" @click="pickZip">
        <span v-if="config.localZipPath">{{ config.localZipPath }}</span>
        <span v-else>点击选择 ZIP 文件</span>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { useWizardNav } from "@/composables/useWizardNav";

const wizard = useWizardStore();
const config = useConfigStore();
const { goTo } = useWizardNav();

const hasBundled = ref(typeof __HAS_BUNDLED__ !== "undefined" ? __HAS_BUNDLED__ : false);
const selected = ref(hasBundled.value ? "bundled" : "online");

watch(selected, (v) => {
  wizard.setSourceMode(v as any);
  wizard.setReady(v !== "local_zip" || !!config.localZipPath);
});

onMounted(() => {
  wizard.setSourceMode(selected.value as any);
  wizard.setReady(selected.value !== "local_zip");
});

async function pickZip() {
  // 简化实现：在真实 Tauri 环境中通过 dialog plugin 选择文件
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".zip";
  input.onchange = () => {
    if (input.files?.[0]) {
      config.localZipPath = input.files[0].name;
      wizard.setReady(true);
    }
  };
  input.click();
}

function handleNext() {
  if (selected.value === "online") {
    goTo("clash-disclaimer");
  } else {
    goTo("install");
  }
}
</script>

<style scoped>
.source-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); }
.options { display: flex; flex-direction: column; gap: 10px; }
.option {
  display: flex; align-items: center; gap: 12px;
  padding: 16px; border: 2px solid var(--color-border);
  border-radius: var(--radius); cursor: pointer;
}
.option.selected { border-color: var(--color-primary); background: #eff6ff; }
.option.disabled { opacity: .5; cursor: not-allowed; }
.opt-icon { font-size: 24px; }
.opt-title { font-weight: 600; }
.opt-desc { font-size: 12px; color: var(--color-muted); margin-top: 2px; }
.unavail { font-size: 11px; color: var(--color-error); font-weight: 400; margin-left: 6px; }
.zip-zone {
  border: 2px dashed var(--color-border); border-radius: var(--radius);
  padding: 24px; text-align: center; cursor: pointer;
  color: var(--color-muted); font-size: 13px;
}
</style>
