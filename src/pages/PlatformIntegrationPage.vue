<template>
  <WizardLayout @next="handleNext" next-label="下一步（可跳过）">
    <div class="platform-page">
      <h2>平台集成（可选）</h2>
      <p class="desc">勾选平台后粘贴 Webhook 地址，机器人即可在群里响应消息。可跳过，安装后在控制台配置。</p>

      <div class="webhook-guide">
        <strong>什么是 Webhook？</strong>
        Webhook 是平台提供给您的一个链接地址，用于将企业群的消息实时推送到 OpenClaw。
        每个平台的群机器人设置页面都有"复制 Webhook"按钮，直接粘贴到下方即可。
        点击各平台的「如何获取？↗」查看图文教程。
      </div>

      <div class="platform-list">
        <div
          v-for="p in platforms"
          :key="p.id"
          class="platform-card"
          :class="{ active: config.platforms[p.id].enabled }"
        >
          <label class="card-toggle">
            <input type="checkbox" v-model="config.platforms[p.id].enabled" />
            <span class="p-icon">{{ p.icon }}</span>
            <span class="p-name">{{ p.name }}</span>
            <span class="toggle-hint" v-if="!config.platforms[p.id].enabled">点击启用</span>
          </label>

          <div v-if="config.platforms[p.id].enabled" class="input-area">
            <div class="input-row">
              <input
                type="url"
                v-model="config.platforms[p.id].webhookUrl"
                :placeholder="p.placeholder"
                class="webhook-input"
                :class="{ valid: isValid(p.id), invalid: hasInput(p.id) && !isValid(p.id) }"
                @paste="onPaste(p.id, $event)"
              />
              <span v-if="isValid(p.id)" class="mark ok">✓</span>
              <span v-else-if="hasInput(p.id)" class="mark warn">!</span>
            </div>
            <div class="hint-row">
              <span class="hint-text">{{ p.hint }}</span>
              <a class="doc-link" @click.prevent="openDocs(p.docUrl)">如何获取？↗</a>
            </div>
            <p v-if="hasInput(p.id) && !isValid(p.id)" class="err-tip">
              地址格式不匹配，请确认是否复制完整
            </p>
          </div>
        </div>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();

const platforms = [
  {
    id: "wx",
    icon: "💼",
    name: "企业微信",
    hint: "在企业微信「群机器人」设置页复制 Webhook 地址",
    docUrl: "https://work.weixin.qq.com/api/doc/90000/90136/91770",
    placeholder: "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=…",
    pattern: /qyapi\.weixin\.qq\.com/,
  },
  {
    id: "qq",
    icon: "🐧",
    name: "QQ 频道",
    hint: "在 QQ 开放平台创建机器人，获取 Webhook 推送地址",
    docUrl: "https://bot.q.qq.com/wiki/develop/webhook/",
    placeholder: "https://qyapi.im.qq.com/cgi-bin/webhook/send?key=…",
    pattern: /qyapi\.im\.qq\.com|bot\.q\.qq\.com/,
  },
  {
    id: "dt",
    icon: "⚙️",
    name: "钉钉",
    hint: "在钉钉群「智能群助手」→「添加机器人」中复制 Webhook",
    docUrl: "https://open.dingtalk.com/document/robots/custom-robot-access",
    placeholder: "https://oapi.dingtalk.com/robot/send?access_token=…",
    pattern: /oapi\.dingtalk\.com/,
  },
  {
    id: "fs",
    icon: "🪁",
    name: "飞书",
    hint: "在飞书群「设置」→「群机器人」→「添加机器人」中获取 Webhook",
    docUrl: "https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot",
    placeholder: "https://open.feishu.cn/open-apis/bot/v2/hook/…",
    pattern: /open\.feishu\.cn/,
  },
];

onMounted(() => { wizard.setReady(true); });

function isValid(id: string) {
  const p = platforms.find((x) => x.id === id)!;
  return p.pattern.test(config.platforms[id].webhookUrl);
}
function hasInput(id: string) {
  return config.platforms[id].webhookUrl.trim().length > 0;
}
function onPaste(id: string, e: ClipboardEvent) {
  // 自动修剪粘贴内容的首尾空格
  const text = e.clipboardData?.getData("text")?.trim();
  if (text) {
    e.preventDefault();
    config.platforms[id].webhookUrl = text;
  }
}
function openDocs(url: string) {
  tauri.openUrl(url).catch(() => { window.open(url, "_blank"); });
}
function handleNext() { next(); }
</script>

<style scoped>
.platform-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); font-size: 13px; }
.webhook-guide {
  background: #f0fdf4; border: 1px solid #bbf7d0;
  border-radius: var(--radius); padding: 12px 14px;
  font-size: 12px; line-height: 1.8; color: #166534;
}

.platform-list { display: flex; flex-direction: column; gap: 10px; }

.platform-card {
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 14px 16px;
  background: var(--color-surface);
  display: flex; flex-direction: column; gap: 12px;
  transition: border-color .15s;
}
.platform-card.active { border-color: var(--color-primary); }

.card-toggle {
  display: flex; align-items: center; gap: 10px; cursor: pointer;
  user-select: none;
}
.card-toggle input[type="checkbox"] { width: 16px; height: 16px; cursor: pointer; }
.p-icon { font-size: 18px; }
.p-name { font-weight: 600; font-size: 14px; }
.toggle-hint { margin-left: auto; font-size: 12px; color: var(--color-muted); }

.input-area { display: flex; flex-direction: column; gap: 6px; }

.input-row { display: flex; align-items: center; gap: 8px; }
.webhook-input {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px;
  transition: border-color .15s;
}
.webhook-input:focus { outline: none; border-color: var(--color-primary); }
.webhook-input.valid { border-color: var(--color-success); }
.webhook-input.invalid { border-color: var(--color-warning); }

.mark { font-size: 16px; font-weight: 700; flex-shrink: 0; }
.mark.ok { color: var(--color-success); }
.mark.warn { color: var(--color-warning); }

.hint-row { display: flex; align-items: center; justify-content: space-between; }
.hint-text { font-size: 12px; color: var(--color-muted); }
.doc-link {
  font-size: 12px; color: var(--color-primary); cursor: pointer;
  text-decoration: none; white-space: nowrap;
}
.doc-link:hover { text-decoration: underline; }

.err-tip { font-size: 12px; color: var(--color-warning); margin: 0; }
</style>
