<template>
  <WizardLayout @next="handleNext" :next-label="testing ? '测试中…' : '使用此代理'">
    <div class="clash-config">
      <h2>配置 Clash 代理</h2>
      <p class="desc">输入您的订阅链接，我们会临时启动 Mihomo 完成资源下载。</p>

      <div class="field">
        <label>订阅链接</label>
        <div class="input-row">
          <input type="url" v-model="subUrl" placeholder="https://your-clash-sub-url..." @input="testResult = null" />
          <button class="btn-secondary" @click="test" :disabled="!subUrl || testing">
            {{ testing ? "测试中…" : "测试连接" }}
          </button>
        </div>
      </div>

      <div class="test-result" v-if="testResult">
        <span v-if="testResult.success" class="ok">✓ 连接成功，延迟 {{ testResult.latency_ms }}ms</span>
        <span v-else class="err">✕ {{ testResult.error }}</span>
      </div>

      <p class="tip">ℹ️ 代理仅在资源下载期间临时启用，完成后自动停止并删除相关文件。</p>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri, type ClashTestResult } from "@/composables/useTauri";

const wizard = useWizardStore();
const config = useConfigStore();
const { next } = useWizardNav();

const subUrl = ref(config.clashSubscriptionUrl || "");
const testing = ref(false);
const testResult = ref<ClashTestResult | null>(null);

watch(subUrl, (v) => { wizard.setReady(!!v); });
wizard.setReady(!!subUrl.value);

async function test() {
  testing.value = true;
  try {
    testResult.value = await tauri.clashTest(subUrl.value);
  } finally {
    testing.value = false;
  }
}

function handleNext() {
  config.clashSubscriptionUrl = subUrl.value;
  next();
}
</script>

<style scoped>
.clash-config { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc, .tip { color: var(--color-muted); font-size: 13px; }
.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
.input-row { display: flex; gap: 8px; }
input[type="url"] {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 13px;
}
.test-result { font-size: 13px; font-weight: 600; }
.ok { color: var(--color-success); }
.err { color: var(--color-error); }
</style>
