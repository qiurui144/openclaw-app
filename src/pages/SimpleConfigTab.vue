<template>
  <div class="config-tab">
    <!-- AI 接入 -->
    <section class="config-section">
      <h3>AI 接入</h3>
      <div class="field-row">
        <label>服务商</label>
        <select v-model="ai.provider" @change="onProviderChange">
          <option value="">-- 选择 --</option>
          <option v-for="p in AI_PROVIDERS" :key="p.value" :value="p.value">{{ p.label }}</option>
        </select>
      </div>
      <div class="field-row" v-if="ai.provider">
        <label>API Base URL</label>
        <input v-model="ai.baseUrl" placeholder="https://api.openai.com/v1" />
      </div>
      <div class="field-row" v-if="ai.provider">
        <label>API Key</label>
        <div class="input-with-action">
          <input v-model="ai.apiKey" :type="showKey ? 'text' : 'password'" placeholder="sk-..." />
          <button class="btn-icon" @click="showKey = !showKey">{{ showKey ? "🙈" : "👁" }}</button>
        </div>
        <a v-if="ai.provider && getKeyUrl(ai.provider)" class="help-link" :href="getKeyUrl(ai.provider)" target="_blank" rel="noopener noreferrer">获取 Key ↗</a>
      </div>
      <div class="field-row" v-if="ai.provider">
        <label>模型</label>
        <input v-model="ai.model" :placeholder="getDefaultModel(ai.provider)" />
      </div>
      <div class="action-row" v-if="ai.provider">
        <button class="btn-primary" @click="testAiConnection" :disabled="testing">
          {{ testing ? "测试中…" : "测试连接" }}
        </button>
        <button class="btn-primary" @click="saveAiConfig" :disabled="saving">
          {{ saving ? "保存中…" : "保存" }}
        </button>
        <span v-if="aiTestResult" :class="['test-result', aiTestResult.ok ? 'ok' : 'fail']">
          {{ aiTestResult.msg }}
        </span>
      </div>
    </section>

    <!-- 聊天平台接入 -->
    <section class="config-section">
      <h3>聊天平台接入</h3>
      <p class="section-desc">点击展开配置。修改后点击「保存」即生效（Gateway 自动热重载）。</p>

      <div v-for="plat in platforms" :key="plat.key" class="platform-item">
        <div class="platform-header" @click="togglePlatform(plat.key)">
          <span class="platform-icon">{{ plat.icon }}</span>
          <span class="platform-name">{{ plat.label }}</span>
          <span class="platform-tag" v-if="plat.tag">{{ plat.tag }}</span>
          <span class="platform-status" v-if="isPlatformConfigured(plat.key)">已配置</span>
          <span class="expand-icon">{{ expandedPlatform === plat.key ? "▼" : "▶" }}</span>
        </div>

        <div v-if="expandedPlatform === plat.key" class="platform-body">
          <p class="platform-guide" v-if="plat.guide">{{ plat.guide }}</p>

          <div v-for="f in plat.fields" :key="f.key" class="field-row">
            <label>{{ f.label }}</label>
            <input
              v-model="channelData[plat.key][f.key]"
              :placeholder="f.placeholder || ''"
              :type="f.secret ? 'password' : 'text'"
            />
          </div>

          <div class="action-row">
            <button class="btn-primary" @click="savePlatform(plat.key)" :disabled="saving">保存</button>
            <button class="btn-secondary" @click="removePlatform(plat.key)" v-if="isPlatformConfigured(plat.key)">移除</button>
          </div>
        </div>
      </div>
    </section>

    <!-- 保存反馈 -->
    <div v-if="saveMsg" class="save-msg" :class="saveMsg.ok ? 'ok' : 'fail'">{{ saveMsg.text }}</div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from "vue";
import { tauri } from "@/composables/useTauri";

// AI 服务商定义
const AI_PROVIDERS = [
  { value: "tongyi", label: "通义千问", baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1", model: "qwen-plus", keyUrl: "https://dashscope.console.aliyun.com/apiKey" },
  { value: "deepseek", label: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", model: "deepseek-chat", keyUrl: "https://platform.deepseek.com/api_keys" },
  { value: "openai", label: "OpenAI", baseUrl: "https://api.openai.com/v1", model: "gpt-4o", keyUrl: "https://platform.openai.com/api-keys" },
  { value: "custom", label: "自定义（OpenAI 兼容）", baseUrl: "", model: "", keyUrl: "" },
];

// 平台定义
interface PlatformField { key: string; label: string; placeholder?: string; secret?: boolean; }
interface PlatformDef {
  key: string; label: string; icon: string; tag?: string; guide?: string;
  fields: PlatformField[];
  toConfig: (data: Record<string, string>) => Record<string, unknown>;
}

const platforms: PlatformDef[] = [
  {
    key: "wecom", label: "企业微信", icon: "💼", tag: "长连接",
    guide: "企业微信管理后台 → 应用管理 → 自建应用，获取 Corp ID、Agent ID 和 Corp Secret。",
    fields: [
      { key: "corpId", label: "Corp ID", placeholder: "ww开头的企业ID" },
      { key: "agentId", label: "Agent ID", placeholder: "应用的 AgentId" },
      { key: "corpSecret", label: "Corp Secret", placeholder: "应用的 Secret", secret: true },
    ],
    toConfig: (d) => ({ agent: { corpId: d.corpId, corpSecret: d.corpSecret, agentId: d.agentId } }),
  },
  {
    key: "dingtalk", label: "钉钉", icon: "🔷", tag: "Stream 长连接",
    guide: "钉钉开放平台 → 应用开发 → 企业内部应用 → 创建应用，获取 Client ID（AppKey）和 Client Secret。",
    fields: [
      { key: "clientId", label: "Client ID (AppKey)", placeholder: "ding开头" },
      { key: "clientSecret", label: "Client Secret", placeholder: "应用 Secret", secret: true },
    ],
    toConfig: (d) => ({ clientId: d.clientId, clientSecret: d.clientSecret }),
  },
  {
    key: "feishu", label: "飞书", icon: "🕊", tag: "WebSocket 长连接",
    guide: "飞书开放平台 → 创建企业自建应用 → 获取 App ID 和 App Secret，启用机器人能力。",
    fields: [
      { key: "appId", label: "App ID", placeholder: "cli_开头" },
      { key: "appSecret", label: "App Secret", placeholder: "应用 Secret", secret: true },
    ],
    toConfig: (d) => ({ appId: d.appId, appSecret: d.appSecret, connectionMode: "websocket" }),
  },
  {
    key: "qqbot", label: "QQ", icon: "🐧", tag: "需公网 IP",
    guide: "QQ 开放平台 → 创建机器人 → 获取 AppID 和 AppSecret，配置回调地址。",
    fields: [
      { key: "appId", label: "App ID" },
      { key: "appSecret", label: "App Secret", secret: true },
    ],
    toConfig: (d) => ({ appId: d.appId, appSecret: d.appSecret, callbackPath: "/webhook/qq" }),
  },
  {
    key: "whatsapp", label: "WhatsApp", icon: "📱", tag: "扫码登录",
    guide: "首次使用需通过 Gateway API 获取 QR 码扫码绑定手机号。",
    fields: [],
    toConfig: () => ({ enabled: true }),
  },
  {
    key: "telegram", label: "Telegram", icon: "✈",
    guide: "与 @BotFather 对话创建 Bot，获取 Bot Token。",
    fields: [
      { key: "botToken", label: "Bot Token", placeholder: "123456:ABC-DEF...", secret: true },
    ],
    toConfig: (d) => ({ botToken: d.botToken }),
  },
  {
    key: "discord", label: "Discord", icon: "🎮",
    guide: "Discord Developer Portal → 创建 Application → Bot → 获取 Token。",
    fields: [
      { key: "botToken", label: "Bot Token", secret: true },
      { key: "applicationId", label: "Application ID" },
    ],
    toConfig: (d) => ({ botToken: d.botToken, applicationId: d.applicationId }),
  },
  {
    key: "slack", label: "Slack", icon: "💬",
    guide: "api.slack.com → 创建 App → 获取 Bot Token 和 App Token。",
    fields: [
      { key: "botToken", label: "Bot Token (xoxb-...)", secret: true },
      { key: "appToken", label: "App Token (xapp-...)", secret: true },
    ],
    toConfig: (d) => ({ botToken: d.botToken, appToken: d.appToken }),
  },
  {
    key: "line", label: "LINE", icon: "🟢",
    guide: "LINE Developers → 创建 Messaging API Channel → 获取 Token 和 Secret。",
    fields: [
      { key: "channelAccessToken", label: "Channel Access Token", secret: true },
      { key: "channelSecret", label: "Channel Secret", secret: true },
    ],
    toConfig: (d) => ({ channelAccessToken: d.channelAccessToken, channelSecret: d.channelSecret }),
  },
];

// 状态
const ai = reactive({ provider: "", baseUrl: "", apiKey: "", model: "" });
const showKey = ref(false);
const testing = ref(false);
const saving = ref(false);
const aiTestResult = ref<{ ok: boolean; msg: string } | null>(null);
const expandedPlatform = ref<string | null>(null);
const saveMsg = ref<{ ok: boolean; text: string } | null>(null);

// 每个平台的表单数据
const channelData: Record<string, Record<string, string>> = reactive({});
for (const p of platforms) {
  channelData[p.key] = {};
  for (const f of p.fields) {
    channelData[p.key][f.key] = "";
  }
}

// 已配置的平台 key 集合
const configuredPlatforms = ref<Set<string>>(new Set());

onMounted(async () => {
  try {
    const cfg = await tauri.readOpenclawConfig();
    // 加载 AI 配置
    const aiCfg = cfg.ai as Record<string, string> | undefined;
    if (aiCfg) {
      ai.provider = aiCfg.provider || "";
      ai.baseUrl = aiCfg.baseUrl || "";
      ai.apiKey = aiCfg.apiKey || "";
      ai.model = aiCfg.model || "";
    }
    // 加载 channel 配置
    const channels = cfg.channels as Record<string, Record<string, unknown>> | undefined;
    if (channels) {
      for (const plat of platforms) {
        const ch = channels[plat.key];
        if (ch && typeof ch === "object" && Object.keys(ch).length > 0) {
          configuredPlatforms.value.add(plat.key);
          // 填充表单（扁平化，处理 wecom 的嵌套 agent 结构）
          if (plat.key === "wecom" && ch.agent && typeof ch.agent === "object") {
            const agent = ch.agent as Record<string, string>;
            channelData.wecom.corpId = agent.corpId || "";
            channelData.wecom.corpSecret = agent.corpSecret || "";
            channelData.wecom.agentId = agent.agentId || "";
          } else {
            for (const f of plat.fields) {
              channelData[plat.key][f.key] = String(ch[f.key] || "");
            }
          }
        }
      }
    }
  } catch { /* ignore */ }
});

function onProviderChange() {
  const p = AI_PROVIDERS.find((x) => x.value === ai.provider);
  if (p) {
    ai.baseUrl = p.baseUrl;
    ai.model = p.model;
  }
}

function getKeyUrl(provider: string): string {
  return AI_PROVIDERS.find((p) => p.value === provider)?.keyUrl || "";
}

function getDefaultModel(provider: string): string {
  return AI_PROVIDERS.find((p) => p.value === provider)?.model || "模型名称";
}

async function testAiConnection() {
  if (!ai.baseUrl || !ai.apiKey) {
    aiTestResult.value = { ok: false, msg: "请填写 API Base URL 和 Key" };
    return;
  }
  testing.value = true;
  aiTestResult.value = null;
  try {
    const resp = await fetch(`${ai.baseUrl}/models`, {
      headers: { Authorization: `Bearer ${ai.apiKey}` },
      signal: AbortSignal.timeout(10000),
    });
    if (resp.ok) {
      aiTestResult.value = { ok: true, msg: "连接成功" };
    } else {
      aiTestResult.value = { ok: false, msg: `HTTP ${resp.status}` };
    }
  } catch (e) {
    aiTestResult.value = { ok: false, msg: `连接失败: ${e}` };
  }
  testing.value = false;
}

async function saveAiConfig() {
  saving.value = true;
  try {
    await tauri.writeOpenclawConfig({
      ai: {
        provider: ai.provider,
        baseUrl: ai.baseUrl,
        apiKey: ai.apiKey,
        model: ai.model,
      },
    });
    showSaveMsg(true, "AI 配置已保存");
  } catch (e) {
    showSaveMsg(false, `保存失败: ${e}`);
  }
  saving.value = false;
}

function togglePlatform(key: string) {
  expandedPlatform.value = expandedPlatform.value === key ? null : key;
}

function isPlatformConfigured(key: string): boolean {
  return configuredPlatforms.value.has(key);
}

async function savePlatform(key: string) {
  const plat = platforms.find((p) => p.key === key);
  if (!plat) return;
  saving.value = true;
  try {
    const config = plat.toConfig(channelData[key]);
    await tauri.writeOpenclawConfig({
      channels: { [key]: config },
    });
    configuredPlatforms.value.add(key);
    showSaveMsg(true, `${plat.label} 配置已保存`);
  } catch (e) {
    showSaveMsg(false, `保存失败: ${e}`);
  }
  saving.value = false;
}

async function removePlatform(key: string) {
  const plat = platforms.find((p) => p.key === key);
  if (!plat) return;
  saving.value = true;
  try {
    // 写入 null 会在 deep merge 中删除该 key
    await tauri.writeOpenclawConfig({
      channels: { [key]: null },
    });
    configuredPlatforms.value.delete(key);
    for (const f of plat.fields) {
      channelData[key][f.key] = "";
    }
    showSaveMsg(true, `${plat.label} 已移除`);
  } catch (e) {
    showSaveMsg(false, `移除失败: ${e}`);
  }
  saving.value = false;
}

function showSaveMsg(ok: boolean, text: string) {
  saveMsg.value = { ok, text };
  setTimeout(() => { saveMsg.value = null; }, 3000);
}
</script>

<style scoped>
.config-tab { display: flex; flex-direction: column; gap: 20px; }

.config-section {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 18px 20px;
}
.config-section h3 { font-size: 15px; font-weight: 700; margin-bottom: 12px; }
.section-desc { font-size: 12px; color: var(--color-muted); margin-bottom: 12px; }

.field-row { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; }
.field-row label { font-size: 12px; font-weight: 500; color: var(--color-muted); }
.field-row input, .field-row select {
  padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; width: 100%;
}
.input-with-action { display: flex; gap: 4px; }
.input-with-action input { flex: 1; }
.btn-icon { padding: 4px 8px; font-size: 14px; background: none; border: 1px solid var(--color-border); border-radius: var(--radius); }
.help-link { font-size: 11px; color: var(--color-primary); text-decoration: none; }
.help-link:hover { text-decoration: underline; }

.action-row { display: flex; gap: 10px; align-items: center; margin-top: 4px; }
.test-result { font-size: 12px; }
.test-result.ok { color: var(--color-success); }
.test-result.fail { color: var(--color-error); }

/* 平台列表 */
.platform-item {
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  margin-bottom: 8px;
  overflow: hidden;
}
.platform-header {
  display: flex; align-items: center; gap: 10px;
  padding: 10px 14px; cursor: pointer;
  transition: background 0.15s;
}
.platform-header:hover { background: var(--color-bg); }
.platform-icon { font-size: 16px; }
.platform-name { font-size: 13px; font-weight: 500; }
.platform-tag {
  font-size: 10px; padding: 1px 6px; border-radius: 4px;
  background: #dbeafe; color: #1e40af;
}
.platform-status {
  font-size: 10px; padding: 1px 6px; border-radius: 4px;
  background: #dcfce7; color: #166534;
}
.expand-icon { margin-left: auto; font-size: 10px; color: var(--color-muted); }

.platform-body { padding: 12px 14px; border-top: 1px solid var(--color-border); background: var(--color-bg); }
.platform-guide { font-size: 12px; color: var(--color-muted); margin-bottom: 12px; line-height: 1.6; }

.save-msg {
  position: fixed; bottom: 24px; right: 24px;
  padding: 10px 20px; border-radius: var(--radius);
  font-size: 13px; font-weight: 500;
  box-shadow: var(--shadow);
}
.save-msg.ok { background: #dcfce7; color: #166534; }
.save-msg.fail { background: #fef2f2; color: #991b1b; }
</style>
