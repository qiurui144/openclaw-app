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
            </label>
            <button v-if="config.wecomEnabled" class="open-btn" @click="openUrl('https://work.weixin.qq.com/wework_admin/frame#apps')">
              🌐 打开企业微信后台
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.wecomEnabled" class="expanded-area">
            <div class="steps-guide">
              <div class="steps-label">操作步骤</div>
              <ol class="step-list">
                <li>登录<strong>企业微信管理后台</strong>（work.weixin.qq.com），进入「应用管理」→「应用」</li>
                <li>点击「创建应用」，类型选「自建」，填写应用名称</li>
                <li>在应用详情页获取 <strong>AgentId</strong>（数字），在「企业信息」页获取 <strong>企业 ID (corpId)</strong></li>
                <li>在应用「Secret」处点击「查看」并获取 <strong>App Secret (corpSecret)</strong>，填入下方</li>
                <li>在「接收消息」→「API 接收」中配置 Token 和 EncodingAESKey（OpenClaw 安装后填入）</li>
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
            </label>
            <button v-if="config.dingtalkEnabled" class="open-btn" @click="openUrl('https://open-dev.dingtalk.com/fe/app')">
              🌐 打开钉钉开放平台
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.dingtalkEnabled" class="expanded-area">
            <div class="info-notice">
              <strong>💡 Stream 长连接模式：</strong>钉钉官方推荐方式，OpenClaw 主动连接钉钉，无需公网 IP 和回调配置。
            </div>
            <div class="steps-guide">
              <div class="steps-label">操作步骤</div>
              <ol class="step-list">
                <li>访问<strong>钉钉开放平台</strong>（open-dev.dingtalk.com），登录后点击「创建应用」→「企业内部应用」→「钉钉应用」</li>
                <li>在「基础信息」页获取 <strong>AppKey (ClientID)</strong> 和 <strong>AppSecret (ClientSecret)</strong></li>
                <li>在「消息推送」→「机器人」中启用机器人，填写名称和描述</li>
                <li>在「消息接收模式」中选择「Stream 模式」（推荐，无需回调地址）</li>
                <li>发布应用后，将应用分享给所需群或人员</li>
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
            </label>
            <button v-if="config.feishuEnabled" class="open-btn" @click="openUrl('https://open.feishu.cn/app')">
              🌐 打开飞书开放平台
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.feishuEnabled" class="expanded-area">
            <div class="info-notice">
              <strong>💡 WebSocket 长连接模式：</strong>无需公网 IP 和回调地址，OpenClaw 主动连接飞书云端，实现双向对话。
            </div>
            <div class="steps-guide">
              <div class="steps-label">操作步骤</div>
              <ol class="step-list">
                <li>访问<strong>飞书开放平台</strong>（open.feishu.cn），登录并点击「创建应用」→「自建应用」</li>
                <li>在「凭证与基础信息」页获取 <strong>App ID</strong> 和 <strong>App Secret</strong>，填入下方</li>
                <li>在「添加应用能力」中启用「机器人」</li>
                <li>在「事件订阅」→「事件配置」中添加 <code>im.message.receive_v1</code>，订阅方式选「<strong>长连接</strong>」</li>
                <li>创建版本并发布上线（企业自建应用通常秒审核）</li>
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
            </label>
            <button v-if="config.qqEnabled" class="open-btn" @click="openUrl('https://bot.q.qq.com/')">
              🌐 打开 QQ 开放平台
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <div v-if="config.qqEnabled" class="expanded-area">
            <div class="warn-notice">
              <strong>⚠️ QQ 使用回调推送模式：</strong>
              需要服务器有公网 IP 或域名，QQ 平台会主动推送消息到 OpenClaw 的回调地址。
            </div>
            <div class="steps-guide">
              <div class="steps-label">操作步骤</div>
              <ol class="step-list">
                <li>访问 <strong>bot.q.qq.com</strong>，登录并创建机器人应用（QQ 账号即可）</li>
                <li>在应用详情页获取 <strong>AppID</strong> 和 <strong>AppSecret</strong>，填入下方</li>
                <li>OpenClaw 安装完成后，将回调地址 <code>{{ callbackUrl }}</code> 填入 QQ 开放平台「回调配置」</li>
                <li>OpenClaw 自动处理 ED25519 签名验证，等待平台审核通过</li>
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

const PLATFORM_URLS: Record<string, string> = {
  wecom:    "https://work.weixin.qq.com/wework_admin/frame#apps",
  dingtalk: "https://open-dev.dingtalk.com/fe/app",
  feishu:   "https://open.feishu.cn/app",
  qq:       "https://bot.q.qq.com/",
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
  tauri.openUrl(url).catch(() => { window.open(url, "_blank"); });
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
.p-badge.blue { background: #eff6ff; color: #3b82f6; }
.p-badge.warn { background: #fffbeb; color: #d97706; }
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
