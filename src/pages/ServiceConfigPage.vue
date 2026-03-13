<template>
  <WizardLayout @next="handleNext">
    <div class="service-config">
      <h2>服务配置</h2>

      <div class="field">
        <label>监听端口</label>
        <input type="number" v-model.number="config.servicePort" min="1024" max="65535" @input="validate" />
        <span class="error" v-if="portError">{{ portError }}</span>
      </div>

      <div class="field">
        <label>绑定域名（可选）</label>
        <input type="text" v-model="config.domainName" placeholder="例：openlaw.example.com" />
        <span class="hint">留空则仅通过 IP:端口 访问</span>
      </div>

      <div class="field">
        <label>管理员密码</label>
        <input type="password" v-model="config.adminPassword" @input="validate" placeholder="至少 8 位，含字母+数字" />
        <PasswordStrength :password="config.adminPassword" />
        <span class="error" v-if="passError">{{ passError }}</span>
      </div>

      <div class="field">
        <label>确认密码</label>
        <input type="password" v-model="config.confirmPassword" @input="validate" />
        <span class="error" v-if="matchError">{{ matchError }}</span>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import PasswordStrength from "@/components/PasswordStrength.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();

const portError = ref("");
const passError = ref("");
const matchError = ref("");

function validate() {
  portError.value = "";
  passError.value = "";
  matchError.value = "";

  const port = config.servicePort;
  if (port < 1024 || port > 65535) {
    portError.value = "端口范围 1024-65535";
    wizard.setReady(false); return;
  }
  if (!config.isPasswordValid) {
    passError.value = "密码至少 8 位，需含字母和数字";
    wizard.setReady(false); return;
  }
  if (!config.passwordsMatch) {
    matchError.value = "两次密码不一致";
    wizard.setReady(false); return;
  }
  wizard.setReady(true);
}

validate();

function handleNext() { next(); }
</script>

<style scoped>
.service-config { display: flex; flex-direction: column; gap: 20px; }
h2 { font-size: 20px; font-weight: 700; }
.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
input[type="number"], input[type="text"], input[type="password"] {
  padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; max-width: 360px;
}
.error { font-size: 12px; color: var(--color-error); }
.hint { font-size: 12px; color: var(--color-muted); }
</style>
