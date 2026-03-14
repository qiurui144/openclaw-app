<template>
  <WizardLayout @next="handleNext" next-label="下一步（可跳过）">
    <div class="platform-page">
      <h2>平台集成（可选）</h2>
      <p class="desc">连接企业协作平台，让机器人直接在群里响应消息。可跳过此步骤，安装后在控制台配置。</p>

      <div class="platform-list">
        <div class="platform-card" v-for="p in platforms" :key="p.id">
          <div class="card-header">
            <label class="checkbox-row">
              <input type="checkbox" v-model="config.platforms[p.id].enabled" />
              <span class="p-icon">{{ p.icon }}</span>
              <span class="p-name">{{ p.name }}</span>
            </label>
          </div>

          <template v-if="config.platforms[p.id].enabled">
            <div class="guide-steps">
              <div class="guide-step">
                <span class="step-num">1</span>
                <span>创建机器人，获取 Webhook 地址</span>
                <div class="guide-btns">
                  <button class="btn-secondary sm" @click="showQr(p)">📱 扫码</button>
                </div>
              </div>
              <div class="guide-step">
                <span class="step-num">2</span>
                <span>将 Webhook 地址粘贴到此处</span>
              </div>
            </div>
            <div class="webhook-input">
              <input type="url" v-model="config.platforms[p.id].webhookUrl" :placeholder="p.placeholder" />
              <span class="valid-mark" v-if="isValidWebhook(p.id)">✓</span>
              <span class="warn-mark" v-else-if="config.platforms[p.id].webhookUrl">⚠</span>
            </div>
          </template>
        </div>
      </div>
    </div>

    <QrCodeModal v-if="activeQr" :url="activeQr" @close="activeQr = null" />
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import QrCodeModal from "@/components/QrCodeModal.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();
const activeQr = ref<string | null>(null);

const platforms = [
  { id: "wx", icon: "💼", name: "企业微信", docUrl: "https://work.weixin.qq.com/api/doc/90000/90136/91770", placeholder: "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=...", pattern: /qyapi\.weixin\.qq\.com/ },
  { id: "qq", icon: "🐧", name: "QQ Work", docUrl: "https://work.qq.com/", placeholder: "https://qyapi.im.qq.com/cgi-bin/webhook/send?key=...", pattern: /qyapi\.im\.qq\.com/ },
  { id: "dt", icon: "⚙️", name: "钉钉", docUrl: "https://open.dingtalk.com/document/robots/custom-robot-access", placeholder: "https://oapi.dingtalk.com/robot/send?access_token=...", pattern: /oapi\.dingtalk\.com/ },
  { id: "fs", icon: "🪁", name: "飞书", docUrl: "https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot", placeholder: "https://open.feishu.cn/open-apis/bot/v2/hook/...", pattern: /open\.feishu\.cn/ },
];

onMounted(() => { wizard.setReady(true); });

function showQr(p: typeof platforms[0]) { activeQr.value = p.docUrl; }
function isValidWebhook(id: string) {
  const p = platforms.find((x) => x.id === id)!;
  return p.pattern.test(config.platforms[id].webhookUrl);
}
function handleNext() { next(); }
</script>

<style scoped>
.platform-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); font-size: 13px; }
.platform-list { display: flex; flex-direction: column; gap: 12px; }
.platform-card {
  border: 1px solid var(--color-border); border-radius: var(--radius);
  padding: 16px; background: var(--color-surface);
  display: flex; flex-direction: column; gap: 12px;
}
.card-header .checkbox-row { display: flex; align-items: center; gap: 8px; cursor: pointer; }
.p-icon { font-size: 20px; }
.p-name { font-weight: 600; }
.guide-steps { display: flex; flex-direction: column; gap: 8px; }
.guide-step { display: flex; align-items: center; gap: 10px; font-size: 13px; }
.step-num {
  width: 20px; height: 20px; border-radius: 50%;
  background: var(--color-primary); color: #fff;
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 700; flex-shrink: 0;
}
.guide-btns { margin-left: auto; display: flex; gap: 6px; }
.sm { padding: 4px 10px; font-size: 12px; }
.webhook-input { display: flex; align-items: center; gap: 8px; }
.webhook-input input {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 13px;
}
.valid-mark { color: var(--color-success); font-size: 16px; }
.warn-mark { color: var(--color-warning); font-size: 16px; }
</style>
