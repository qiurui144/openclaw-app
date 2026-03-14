<template>
  <WizardLayout @next="handleNext">
    <div class="service-config">
      <h2>服务配置</h2>
      <p class="step-intro">配置 OpenClaw 网关的网络访问方式和管理员密码。</p>

      <StepHelp>
        <ul>
          <li><strong>监听端口</strong>：OpenClaw 网页控制台的访问端口。安装完成后通过 <code>http://您的IP:端口</code> 访问管理界面。默认 18789，可按需修改（1024–65535）。</li>
          <li><strong>绑定域名（可选）</strong>：如您有域名（如 <code>bot.example.com</code>），填入后向导会在配置文件中记录，方便后续配置反向代理和 HTTPS。可留空，仅通过 IP:端口访问。</li>
          <li><strong>管理员密码</strong>：登录网页控制台所需密码，请设置至少 8 位包含字母和数字的强密码。忘记后可通过编辑配置文件重置。</li>
        </ul>
        <div class="tip">💡 如需外网访问，建议配置域名并通过 Nginx 添加 HTTPS，不要直接暴露端口。</div>
      </StepHelp>

      <div class="field">
        <label>监听端口</label>
        <input type="number" v-model.number="config.servicePort" min="1024" max="65535" @input="validate" />
        <span class="hint">安装后访问地址：http://127.0.0.1:{{ config.servicePort }}</span>
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
import StepHelp from "@/components/StepHelp.vue";
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
.step-intro { font-size: 13px; color: var(--color-muted); margin-top: -12px; }
.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
input[type="number"], input[type="text"], input[type="password"] {
  padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; max-width: 360px;
}
.error { font-size: 12px; color: var(--color-error); }
.hint { font-size: 12px; color: var(--color-muted); }
</style>
