<template>
  <WizardLayout @next="handleNext" next-label="下一步（可跳过）">
    <div class="platform-page">
      <h2>平台集成（可选）</h2>
      <p class="desc">
        所有平台均通过应用凭据接入，支持 WebSocket/Stream 长连接，<strong>无需公网 IP</strong>（QQ 除外）。
        可跳过，安装后在控制台配置。
      </p>

      <div class="platform-list">

        <!-- 企业微信 -->
        <div class="platform-card" :class="{ active: config.wecomEnabled }">
          <div class="card-header">
            <label class="card-toggle">
              <input type="checkbox" v-model="config.wecomEnabled" @change="onToggle('wecom')" />
              <span class="p-icon">💼</span>
              <span class="p-name">企业微信</span>
              <span class="p-badge">自建应用</span>
              <span class="p-badge admin">仅企业管理员</span>
            </label>
            <button v-if="config.wecomEnabled" class="open-btn" @click="openUrl(PLATFORM_URLS.wecom)">
              🌐 企业微信接入向导
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.wecomEnabled" class="expanded-area">
            <div class="warn-notice">
              <strong>⚠️ 需要企业管理员权限：</strong>
              企业微信需使用管理员账号登录企业微信后台，获取企业 ID 和应用凭据，个人账号无法完成此配置。
            </div>
            <div class="steps-guide">
              <div class="steps-label">操作步骤</div>
              <ol class="step-list">
                <li>以<strong>企业管理员</strong>身份登录企业微信管理后台（work.weixin.qq.com），进入「应用管理」→「应用」</li>
                <li>点击「创建应用」→「自建」，填写应用名称后创建</li>
                <li>在应用详情页获取 <strong>AgentId</strong>（数字），在「我的企业」→「企业信息」获取 <strong>企业 ID (corpId)</strong></li>
                <li>在应用详情页点击「Secret」→「查看」，获取 <strong>corpSecret</strong>，填入下方</li>
                <li>OpenClaw 采用长连接模式，<strong>无需公网 IP</strong>，安装完成后自动建立连接</li>
              </ol>
            </div>
            <div class="input-section">
              <div class="field-row">
                <div class="field">
                  <div class="input-label">企业 ID (corpId)</div>
                  <input type="text" v-model="config.wecomCorpId" placeholder="如：ww1234567890abcdef" class="cred-input" />
                </div>
                <div class="field">
                  <div class="input-label">应用 AgentId</div>
                  <input type="text" v-model="config.wecomAgentId" placeholder="如：1000001" class="cred-input" />
                </div>
              </div>
              <div class="field">
                <div class="input-label">应用 Secret (corpSecret)</div>
                <div class="secret-row">
                  <input
                    :type="showWecomSecret ? 'text' : 'password'"
                    v-model="config.wecomCorpSecret"
                    placeholder="在应用 Secret 处查看并复制"
                    class="cred-input"
                  />
                  <button class="eye-btn" @click="showWecomSecret = !showWecomSecret">{{ showWecomSecret ? "🙈" : "👁️" }}</button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 钉钉 -->
        <div class="platform-card" :class="{ active: config.dingtalkEnabled }">
          <div class="card-header">
            <label class="card-toggle">
              <input type="checkbox" v-model="config.dingtalkEnabled" @change="onToggle('dingtalk')" />
              <span class="p-icon">⚙️</span>
              <span class="p-name">钉钉</span>
              <span class="p-badge blue">Stream 长连接</span>
              <span class="p-badge personal">个人可用</span>
            </label>
            <button v-if="config.dingtalkEnabled" class="open-btn" @click="openUrl(PLATFORM_URLS.dingtalk)">
              🌐 钉钉接入文档
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.dingtalkEnabled" class="expanded-area">
            <div class="info-notice">
              <strong>💡 个人开发者可直接接入：</strong>无需企业管理员，使用个人钉钉账号在开放平台创建应用即可。Stream 长连接模式下无需公网 IP。
            </div>
            <div class="steps-guide">
              <div class="steps-label">操作步骤（参考钉钉官方 OpenClaw 接入文档）</div>
              <ol class="step-list">
                <li>访问<strong>钉钉开放平台</strong>（open.dingtalk.com），使用<strong>个人钉钉账号</strong>登录，点击「创建应用」→「企业内部应用」→「钉钉应用」</li>
                <li>在「基础信息」获取 <strong>AppKey (ClientID)</strong> 和 <strong>AppSecret (ClientSecret)</strong>，填入下方</li>
                <li>在「消息推送」→「机器人」启用机器人；消息接收模式选「<strong>Stream 模式</strong>」（推荐）</li>
                <li>应用创建后即可在开发态使用，<strong>正式发布</strong>给组织成员需组织管理员审批</li>
              </ol>
            </div>
            <div class="input-section">
              <div class="field-row">
                <div class="field">
                  <div class="input-label">ClientID (AppKey)</div>
                  <input type="text" v-model="config.dingtalkClientId" placeholder="如：dingxxxxxx" class="cred-input" />
                </div>
                <div class="field">
                  <div class="input-label">ClientSecret (AppSecret)</div>
                  <div class="secret-row">
                    <input
                      :type="showDingtalkSecret ? 'text' : 'password'"
                      v-model="config.dingtalkClientSecret"
                      placeholder="在基础信息页面复制"
                      class="cred-input"
                    />
                    <button class="eye-btn" @click="showDingtalkSecret = !showDingtalkSecret">{{ showDingtalkSecret ? "🙈" : "👁️" }}</button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 飞书 -->
        <div class="platform-card" :class="{ active: config.feishuEnabled }">
          <div class="card-header">
            <label class="card-toggle">
              <input type="checkbox" v-model="config.feishuEnabled" @change="onToggle('feishu')" />
              <span class="p-icon">🪁</span>
              <span class="p-name">飞书</span>
              <span class="p-badge blue">WebSocket 长连接</span>
              <span class="p-badge semi">需飞书企业账号</span>
            </label>
            <button v-if="config.feishuEnabled" class="open-btn" @click="openUrl(PLATFORM_URLS.feishu)">
              🌐 飞书接入专题
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.feishuEnabled" class="expanded-area">
            <div class="info-notice">
              <strong>💡 WebSocket 长连接，无需公网 IP：</strong>需要飞书企业账号（个人账号可免费创建测试企业）。企业管理员审批后可正式发布给成员使用。
            </div>
            <div class="steps-guide">
              <div class="steps-label">操作步骤（参考飞书 OpenClaw 接入专题）</div>
              <ol class="step-list">
                <li>访问<strong>飞书开放平台</strong>（open.feishu.cn），使用<strong>飞书企业账号</strong>登录（无企业可免费创建测试企业）</li>
                <li>点击「创建应用」→「自建应用」，在「凭证与基础信息」获取 <strong>App ID</strong> 和 <strong>App Secret</strong>，填入下方</li>
                <li>在「添加应用能力」启用「机器人」；在「事件订阅」→「事件配置」添加 <code>im.message.receive_v1</code>，订阅方式选「<strong>长连接</strong>」</li>
                <li>申请权限并创建版本发布，<strong>正式上线</strong>需企业管理员审批（测试成员无需审批即可使用）</li>
              </ol>
            </div>
            <div class="input-section">
              <div class="field-row">
                <div class="field">
                  <div class="input-label">App ID</div>
                  <input type="text" v-model="config.feishuAppId" placeholder="如：cli_xxxxxxxx" class="cred-input" />
                </div>
                <div class="field">
                  <div class="input-label">App Secret</div>
                  <div class="secret-row">
                    <input
                      :type="showFeishuSecret ? 'text' : 'password'"
                      v-model="config.feishuAppSecret"
                      placeholder="在凭证与基础信息页面复制"
                      class="cred-input"
                    />
                    <button class="eye-btn" @click="showFeishuSecret = !showFeishuSecret">{{ showFeishuSecret ? "🙈" : "👁️" }}</button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- QQ 机器人 -->
        <div class="platform-card" :class="{ active: config.qqEnabled }">
          <div class="card-header">
            <label class="card-toggle">
              <input type="checkbox" v-model="config.qqEnabled" @change="onToggle('qq')" />
              <span class="p-icon">🐧</span>
              <span class="p-name">QQ 机器人</span>
              <span class="p-badge warn">需公网回调</span>
              <span class="p-badge personal">个人可用</span>
            </label>
            <button v-if="config.qqEnabled" class="open-btn" @click="openUrl(PLATFORM_URLS.qq)">
              🌐 QQ 机器人接入
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.qqEnabled" class="expanded-area">
            <div class="warn-notice">
              <strong>⚠️ 个人可用，但需公网 IP：</strong>
              使用普通 QQ 账号即可在开放平台创建机器人。QQ 平台采用回调推送，服务器需有公网 IP 或域名。
            </div>
            <div class="steps-guide">
              <div class="steps-label">操作步骤（通过 QQ 机器人 OpenClaw 专属页）</div>
              <ol class="step-list">
                <li>访问 <strong>q.qq.com/qqbot/openclaw/login.html</strong>，使用 <strong>QQ 账号</strong>（无需企业资质）授权登录</li>
                <li>按页面引导创建机器人，获取 <strong>AppID</strong> 和 <strong>AppSecret</strong>，填入下方</li>
                <li>OpenClaw 安装完成后，将回调地址 <code>{{ callbackUrl }}</code> 填入「回调配置」</li>
                <li>OpenClaw 自动处理 ED25519 签名验证；审核通过前仅沙箱测试成员可用</li>
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
                  <div class="secret-row">
                    <input
                      :type="showQqSecret ? 'text' : 'password'"
                      v-model="config.qqAppSecret"
                      placeholder="在开放平台复制（只显示一次，请立即保存）"
                      class="cred-input"
                    />
                    <button class="eye-btn" @click="showQqSecret = !showQqSecret">{{ showQqSecret ? "🙈" : "👁️" }}</button>
                  </div>
                </div>
              </div>
              <div class="callback-row">
                <span class="input-label">安装后的回调地址：</span>
                <code class="callback-url">{{ callbackUrl }}</code>
                <button class="copy-btn" @click="copyCallback">复制</button>
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

const showWecomSecret    = ref(false);
const showDingtalkSecret = ref(false);
const showFeishuSecret   = ref(false);
const showQqSecret       = ref(false);

// 各平台 OpenClaw 专属接入页（优先使用平台官方专属页，引导用户完成接入）
const PLATFORM_URLS: Record<string, string> = {
  wecom:    "https://work.weixin.qq.com/nl/index/openclaw",       // 企业微信 OpenClaw 专属页
  dingtalk: "https://open.dingtalk.com/document/dingstart/install-openclaw-locally", // 钉钉官方 OpenClaw 文档
  feishu:   "https://www.feishu.cn/content/topic/openclaw",       // 飞书 OpenClaw 专题汇总页
  qq:       "https://q.qq.com/qqbot/openclaw/login.html",         // QQ 机器人 OpenClaw 专属登录页
};

const callbackUrl = computed(() => {
  const base = config.domainName
    ? `https://${config.domainName}`
    : `http://YOUR_SERVER_IP:${config.servicePort}`;
  return `${base}/webhook/qq`;
});

onMounted(() => { wizard.setReady(true); });

function onToggle(platform: string) {
  const url = PLATFORM_URLS[platform];
  if (!url) return;
  const enabled =
    platform === "wecom" ? config.wecomEnabled :
    platform === "dingtalk" ? config.dingtalkEnabled :
    platform === "feishu" ? config.feishuEnabled :
    config.qqEnabled;
  if (enabled) openUrl(url);
}

function openUrl(url: string) {
  tauri.openUrl(url).catch(() => {}); // 不能用 window.open，Tauri WebView 会自行导航到外部 URL
}

function copyCallback() {
  navigator.clipboard.writeText(callbackUrl.value).catch(() => {});
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

.card-header { display: flex; align-items: center; gap: 10px; }
.card-toggle {
  display: flex; align-items: center; gap: 10px; cursor: pointer;
  user-select: none; flex: 1;
}
.card-toggle input[type="checkbox"] { width: 16px; height: 16px; cursor: pointer; }
.p-icon { font-size: 18px; }
.p-name { font-weight: 600; font-size: 14px; }
.p-badge {
  margin-left: 6px; background: #f1f5f9; color: #64748b;
  font-size: 11px; padding: 1px 7px; border-radius: 10px; font-weight: 600;
}
.p-badge.blue   { background: #eff6ff; color: #3b82f6; }
.p-badge.warn   { background: #fffbeb; color: #d97706; }
.p-badge.admin  { background: #fef2f2; color: #dc2626; }
.p-badge.personal { background: #f0fdf4; color: #16a34a; }
.p-badge.semi   { background: #fefce8; color: #ca8a04; }
.toggle-hint { margin-left: auto; font-size: 12px; color: var(--color-muted); }

.open-btn {
  flex-shrink: 0;
  padding: 5px 12px; font-size: 12px; font-weight: 500;
  border: 1px solid var(--color-primary); border-radius: var(--radius);
  background: #eff6ff; color: var(--color-primary); cursor: pointer;
  white-space: nowrap; transition: background .15s;
}
.open-btn:hover { background: var(--color-primary); color: #fff; }

.expanded-area { display: flex; flex-direction: column; gap: 12px; }

.steps-guide {
  background: #f8fafc; border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 12px 14px;
}
.steps-label {
  font-size: 12px; font-weight: 600; color: var(--color-muted);
  margin-bottom: 8px; text-transform: uppercase; letter-spacing: .5px;
}
.step-list { margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 5px; }
.step-list li { font-size: 13px; line-height: 1.6; color: #334155; }

.info-notice {
  background: #eff6ff; border: 1px solid #bfdbfe;
  border-radius: var(--radius); padding: 9px 13px;
  font-size: 12px; line-height: 1.6; color: #1d4ed8;
}
.warn-notice {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 9px 13px;
  font-size: 12px; line-height: 1.6; color: #92400e;
}

.input-section { display: flex; flex-direction: column; gap: 8px; }
.input-label { font-size: 12px; font-weight: 500; color: var(--color-muted); margin-bottom: 3px; }
.field-row { display: flex; gap: 12px; }
.field { display: flex; flex-direction: column; flex: 1; }
.secret-row { display: flex; gap: 6px; align-items: center; }
.cred-input {
  flex: 1; padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; width: 100%; box-sizing: border-box;
}
.cred-input:focus { outline: none; border-color: var(--color-primary); }
.eye-btn {
  flex-shrink: 0; background: none; border: 1px solid var(--color-border);
  border-radius: var(--radius); cursor: pointer; font-size: 14px; padding: 6px 8px;
}

.callback-row {
  display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
  font-size: 12px; color: var(--color-muted);
}
.callback-url {
  background: #1e293b; color: #94a3b8;
  border-radius: 4px; padding: 2px 8px; font-family: monospace; font-size: 12px;
  word-break: break-all;
}
.copy-btn {
  padding: 2px 8px; font-size: 11px; cursor: pointer;
  border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-surface);
}
</style>
