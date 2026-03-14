<template>
  <WizardLayout @next="handleNext" next-label="下一步（可跳过）">
    <div class="platform-page">
      <h2>平台集成（可选）</h2>
      <p class="desc">勾选平台后，浏览器将自动打开配置页，按应用内步骤操作后填入凭据。可跳过，安装后在控制台配置。</p>

      <div class="platform-list">
        <!-- 企业微信 / 钉钉 / 飞书：Webhook URL 方式 -->
        <div
          v-for="p in webhookPlatforms"
          :key="p.id"
          class="platform-card"
          :class="{ active: config.platforms[p.id].enabled }"
        >
          <div class="card-header">
            <label class="card-toggle">
              <input type="checkbox" v-model="config.platforms[p.id].enabled" @change="onToggle(p)" />
              <span class="p-icon">{{ p.icon }}</span>
              <span class="p-name">{{ p.name }}</span>
            </label>
            <button v-if="config.platforms[p.id].enabled" class="open-btn" @click="openPlatform(p)">
              🌐 打开{{ p.shortName }}配置页
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.platforms[p.id].enabled" class="expanded-area">
            <div class="steps-guide">
              <div class="steps-label">操作步骤（在客户端中）</div>
              <ol class="step-list">
                <li v-for="(step, i) in p.steps" :key="i">{{ step }}</li>
              </ol>
            </div>
            <div class="input-section">
              <div class="input-label">粘贴 Webhook 地址</div>
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
              <p v-if="hasInput(p.id) && !isValid(p.id)" class="err-tip">
                地址格式不匹配，请确认是否复制完整
              </p>
            </div>
          </div>
        </div>

        <!-- QQ 机器人：AppID + AppSecret 方式（不同于其他平台） -->
        <div class="platform-card qq-card" :class="{ active: config.qqEnabled }">
          <div class="card-header">
            <label class="card-toggle">
              <input type="checkbox" v-model="config.qqEnabled" @change="onToggleQq" />
              <span class="p-icon">🐧</span>
              <span class="p-name">QQ 机器人</span>
              <span class="p-badge">回调模式</span>
            </label>
            <button v-if="config.qqEnabled" class="open-btn" @click="openQqPlatform">
              🌐 打开 QQ 开放平台
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.qqEnabled" class="expanded-area">
            <!-- QQ 与其他平台的差异说明 -->
            <div class="qq-notice">
              <strong>⚠️ QQ 机器人与其他平台不同：</strong>
              您需要在 QQ 开放平台注册应用，将 OpenClaw 的回调地址填入平台，由平台主动推送消息到您的服务。
            </div>

            <div class="steps-guide">
              <div class="steps-label">操作步骤（在浏览器中）</div>
              <ol class="step-list">
                <li>访问 <strong>bot.q.qq.com</strong>，登录并创建机器人应用</li>
                <li>在应用详情页获取 <strong>AppID</strong> 和 <strong>AppSecret</strong>，填入下方</li>
                <li>在「回调配置」中，将 OpenClaw 的回调地址填入：<br>
                  <code class="callback-url">{{ callbackUrl }}</code>
                  <button class="copy-btn" @click="copyCallback">复制</button>
                </li>
                <li>完成 ED25519 签名验证（OpenClaw 自动处理），等待审核通过</li>
              </ol>
            </div>

            <div class="input-section">
              <div class="field-row">
                <div class="field">
                  <div class="input-label">AppID</div>
                  <input type="text" v-model="config.qqAppId" placeholder="如：12345678" class="cred-input" />
                </div>
                <div class="field">
                  <div class="input-label">AppSecret</div>
                  <input
                    :type="showQqSecret ? 'text' : 'password'"
                    v-model="config.qqAppSecret"
                    placeholder="在开放平台复制"
                    class="cred-input"
                  />
                  <button class="eye-inline" @click="showQqSecret = !showQqSecret">{{ showQqSecret ? "🙈" : "👁️" }}</button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();
const showQqSecret = ref(false);

// QQ 回调地址：基于配置的端口/域名生成
const callbackUrl = computed(() => {
  const base = config.domainName
    ? `https://${config.domainName}`
    : `http://YOUR_SERVER_IP:${config.servicePort}`;
  return `${base}/webhook/qq`;
});

interface WebhookPlatform {
  id: string;
  icon: string;
  name: string;
  shortName: string;
  placeholder: string;
  pattern: RegExp;
  configUrl: string;
  steps: string[];
}

const webhookPlatforms: WebhookPlatform[] = [
  {
    id: "wx",
    icon: "💼",
    name: "企业微信",
    shortName: "企业微信",
    placeholder: "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=…",
    pattern: /qyapi\.weixin\.qq\.com/,
    // 打开企业微信管理后台（需已登录），可在群聊中设置机器人
    configUrl: "https://work.weixin.qq.com/api/doc/90000/90136/91770",
    steps: [
      "打开企业微信客户端，进入目标群聊",
      "点击右上角「…」→「群机器人」→「添加机器人」",
      "填写机器人名称，点击「添加」确认",
      "在机器人详情页找到「Webhook 地址」，点击「复制」",
      "回到此处，将地址粘贴到下方输入框",
    ],
  },
  {
    id: "dt",
    icon: "⚙️",
    name: "钉钉",
    shortName: "钉钉",
    placeholder: "https://oapi.dingtalk.com/robot/send?access_token=…",
    pattern: /oapi\.dingtalk\.com/,
    // 打开官方文档，步骤截图完整，实际操作在钉钉客户端完成
    configUrl: "https://open.dingtalk.com/document/robots/custom-robot-access",
    steps: [
      "打开钉钉客户端，进入目标群聊",
      "点击右上角「…」→「智能群助手」→「添加机器人」→「自定义」",
      "填写机器人名称，安全设置选「加签」（推荐）",
      "点击「完成」，复制页面中的「Webhook 地址」",
      "回到此处，将地址粘贴到下方输入框",
    ],
  },
  {
    id: "fs",
    icon: "🪁",
    name: "飞书",
    shortName: "飞书",
    placeholder: "https://open.feishu.cn/open-apis/bot/v2/hook/…",
    pattern: /open\.feishu\.cn/,
    // 打开官方文档，实际操作在飞书客户端群设置完成
    configUrl: "https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot",
    steps: [
      "打开飞书客户端，进入目标群聊",
      "点击右上角「…」→「设置」→「群机器人」→「添加机器人」",
      "搜索并选择「自定义机器人」，填写名称后点击「添加」",
      "复制弹出窗口中的「Webhook 地址」",
      "回到此处，将地址粘贴到下方输入框",
    ],
  },
];

onMounted(() => { wizard.setReady(true); });

/** Webhook 平台：勾选时自动打开浏览器 */
function onToggle(p: WebhookPlatform) {
  if (config.platforms[p.id].enabled) openPlatform(p);
}

function openPlatform(p: WebhookPlatform) {
  tauri.openUrl(p.configUrl).catch(() => { window.open(p.configUrl, "_blank"); });
}

/** QQ：勾选时打开 QQ 开放平台 */
function onToggleQq() {
  if (config.qqEnabled) openQqPlatform();
}

function openQqPlatform() {
  const url = "https://bot.q.qq.com/";
  tauri.openUrl(url).catch(() => { window.open(url, "_blank"); });
}

function copyCallback() {
  navigator.clipboard.writeText(callbackUrl.value).catch(() => {});
}

function isValid(id: string) {
  const p = platforms.find((x) => x.id === id)!;
  return p.pattern.test(config.platforms[id].webhookUrl);
}
function hasInput(id: string) {
  return config.platforms[id].webhookUrl.trim().length > 0;
}
function onPaste(id: string, e: ClipboardEvent) {
  const text = e.clipboardData?.getData("text")?.trim();
  if (text) {
    e.preventDefault();
    config.platforms[id].webhookUrl = text;
  }
}
function handleNext() { next(); }
</script>

<style scoped>
.platform-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); font-size: 13px; }

.platform-list { display: flex; flex-direction: column; gap: 10px; }

.platform-card {
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 14px 16px;
  background: var(--color-surface);
  display: flex; flex-direction: column; gap: 14px;
  transition: border-color .15s;
}
.platform-card.active { border-color: var(--color-primary); }

/* 标题行 */
.card-header {
  display: flex; align-items: center; gap: 10px;
}
.card-toggle {
  display: flex; align-items: center; gap: 10px; cursor: pointer;
  user-select: none; flex: 1;
}
.card-toggle input[type="checkbox"] { width: 16px; height: 16px; cursor: pointer; }
.p-icon { font-size: 18px; }
.p-name { font-weight: 600; font-size: 14px; }
.toggle-hint { margin-left: auto; font-size: 12px; color: var(--color-muted); }

/* 打开浏览器按钮 */
.open-btn {
  flex-shrink: 0;
  padding: 5px 12px; font-size: 12px; font-weight: 500;
  border: 1px solid var(--color-primary); border-radius: var(--radius);
  background: #eff6ff; color: var(--color-primary); cursor: pointer;
  white-space: nowrap; transition: background .15s;
}
.open-btn:hover { background: var(--color-primary); color: #fff; }

/* 展开区 */
.expanded-area { display: flex; flex-direction: column; gap: 12px; }

/* 步骤指引 */
.steps-guide {
  background: #f8fafc; border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 12px 14px;
}
.steps-label {
  font-size: 12px; font-weight: 600; color: var(--color-muted);
  margin-bottom: 8px; text-transform: uppercase; letter-spacing: .5px;
}
.step-list {
  margin: 0; padding-left: 20px;
  display: flex; flex-direction: column; gap: 5px;
}
.step-list li { font-size: 13px; line-height: 1.6; color: #334155; }

/* 输入区 */
.input-section { display: flex; flex-direction: column; gap: 6px; }
.input-label { font-size: 12px; font-weight: 500; color: var(--color-muted); }

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

.err-tip { font-size: 12px; color: var(--color-warning); margin: 0; }

/* QQ 专有样式 */
.qq-card .p-badge {
  margin-left: 6px; background: #f0e6ff; color: #7c3aed;
  font-size: 11px; padding: 1px 7px; border-radius: 10px; font-weight: 600;
}
.qq-notice {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 10px 14px;
  font-size: 12px; line-height: 1.7; color: #92400e;
}
.callback-url {
  display: inline-block; background: #1e293b; color: #94a3b8;
  border-radius: 4px; padding: 2px 8px; font-family: monospace; font-size: 12px;
  word-break: break-all;
}
.copy-btn {
  margin-left: 8px; padding: 2px 8px; font-size: 11px; cursor: pointer;
  border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-surface);
}
.field-row { display: flex; gap: 12px; }
.field { display: flex; flex-direction: column; gap: 4px; flex: 1; position: relative; }
.cred-input {
  padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; width: 100%; box-sizing: border-box;
}
.eye-inline {
  position: absolute; right: 8px; top: 26px;
  background: none; border: none; cursor: pointer; font-size: 14px;
}
</style>
