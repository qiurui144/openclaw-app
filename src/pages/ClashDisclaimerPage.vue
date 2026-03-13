<template>
  <WizardLayout @next="handleNext">
    <div class="disclaimer-page">
      <h2>⚠️ 代理工具免责声明</h2>

      <div class="disclaimer-box">
        <p>本向导可选择性地集成 <strong>Mihomo（原 Clash Meta）</strong> 代理工具，仅用于在网络受限环境下下载 OpenClaw 所需资源。</p>
        <br />
        <p><strong>使用前请确认以下事项：</strong></p>
        <ul>
          <li>您已充分了解并遵守您所在地区关于网络代理的相关法律法规。</li>
          <li>代理订阅链接由您自行提供，本软件不提供、推荐或背书任何代理服务。</li>
          <li>代理仅在资源下载期间临时运行，完成后将自动停止并删除相关文件。</li>
          <li>Mihomo 使用 GPL-3.0 开源协议，来源：MetaCubX/mihomo。</li>
          <li>因使用代理工具产生的一切法律责任由用户自行承担。</li>
        </ul>
        <br />
        <p>若您所在网络无限制，可点击"跳过代理，直连下载"。</p>
      </div>

      <label class="accept-row">
        <input type="checkbox" v-model="accepted" />
        <span>我已阅读并同意上述免责声明，继续配置代理</span>
      </label>

      <div class="skip-row">
        <button class="btn-secondary" @click="skipClash">跳过代理，直连下载</button>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const wizard = useWizardStore();
const { next, goTo } = useWizardNav();
const accepted = ref(false);

watch(accepted, (v) => {
  wizard.setClashAccepted(v);
  wizard.setReady(v);
});

function handleNext() {
  if (accepted.value) next();
}

function skipClash() {
  wizard.setClashAccepted(false);
  goTo("install");
}
</script>

<style scoped>
.disclaimer-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; color: var(--color-warning); }
.disclaimer-box {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 16px; font-size: 13px; line-height: 1.8;
}
.disclaimer-box ul { padding-left: 20px; }
.accept-row { display: flex; gap: 10px; align-items: flex-start; cursor: pointer; font-size: 13px; }
.skip-row { margin-top: -4px; }
</style>
