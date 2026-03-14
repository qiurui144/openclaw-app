<template>
  <WizardLayout @next="handleNext" next-label="下一步（可跳过）">
    <div class="platform-page">
      <h2>平台集成（可选）</h2>
      <p class="desc">勾选平台后，浏览器将自动打开对应的机器人配置页面，按应用内步骤获取 Webhook 地址后粘贴即可。</p>

      <div class="platform-list">
        <div
          v-for="p in platforms"
          :key="p.id"
          class="platform-card"
          :class="{ active: config.platforms[p.id].enabled }"
        >
          <!-- 标题行：勾选框 + 图标 + 名称 + 打开浏览器按钮 -->
          <div class="card-header">
            <label class="card-toggle">
              <input
                type="checkbox"
                v-model="config.platforms[p.id].enabled"
                @change="onToggle(p)"
              />
              <span class="p-icon">{{ p.icon }}</span>
              <span class="p-name">{{ p.name }}</span>
            </label>

            <button
              v-if="config.platforms[p.id].enabled"
              class="open-btn"
              @click="openPlatform(p)"
              title="在浏览器中打开配置页"
            >
              🌐 打开{{ p.shortName }}配置页
            </button>
            <span v-else class="toggle-hint">点击启用</span>
          </div>

          <!-- 展开区：步骤 + 输入框 -->
          <div v-if="config.platforms[p.id].enabled" class="expanded-area">

            <!-- 分步操作指引 -->
            <div class="steps-guide">
              <div class="steps-label">操作步骤（浏览器中）</div>
              <ol class="step-list">
                <li v-for="(step, i) in p.steps" :key="i">{{ step }}</li>
              </ol>
            </div>

            <!-- Webhook 输入 -->
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

interface Platform {
  id: string;
  icon: string;
  name: string;
  shortName: string;
  placeholder: string;
  pattern: RegExp;
  configUrl: string;   // 在浏览器中打开的配置页
  steps: string[];     // 应用内分步操作指引
}

const platforms: Platform[] = [
  {
    id: "wx",
    icon: "💼",
    name: "企业微信",
    shortName: "企业微信",
    placeholder: "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=…",
    pattern: /qyapi\.weixin\.qq\.com/,
    configUrl: "https://work.weixin.qq.com/wework_admin/frame#/robot",
    steps: [
      "登录企业微信管理后台",
      "进入目标群聊 → 右上角「…」→「群机器人」",
      "点击「添加机器人」→ 填写机器人名称 → 确认",
      "在机器人详情页找到「Webhook 地址」，点击「复制」",
      "回到此页面，将地址粘贴到下方输入框",
    ],
  },
  {
    id: "qq",
    icon: "🐧",
    name: "QQ 频道机器人",
    shortName: "QQ",
    placeholder: "https://qyapi.im.qq.com/cgi-bin/webhook/send?key=…",
    pattern: /qyapi\.im\.qq\.com|bot\.q\.qq\.com/,
    configUrl: "https://bot.q.qq.com/#/developer/developer-setting",
    steps: [
      "登录 QQ 机器人开放平台（bot.q.qq.com）",
      "进入「我的机器人」→ 选择或新建机器人",
      "在「Webhook」设置中启用 Webhook 推送",
      "复制系统生成的「Webhook 接收地址」",
      "回到此页面，将地址粘贴到下方输入框",
    ],
  },
  {
    id: "dt",
    icon: "⚙️",
    name: "钉钉",
    shortName: "钉钉",
    placeholder: "https://oapi.dingtalk.com/robot/send?access_token=…",
    pattern: /oapi\.dingtalk\.com/,
    configUrl: "https://open-dev.dingtalk.com/fe/app#/corp/robot",
    steps: [
      "打开钉钉 → 进入目标群 → 右上角「…」→「智能群助手」",
      "点击「添加机器人」→ 选择「自定义」→ 填写机器人名称",
      "安全设置选择「加签」或「自定义关键词」（按需）",
      "完成后在「Webhook 地址」一栏点击「复制」",
      "回到此页面，将地址粘贴到下方输入框",
    ],
  },
  {
    id: "fs",
    icon: "🪁",
    name: "飞书",
    shortName: "飞书",
    placeholder: "https://open.feishu.cn/open-apis/bot/v2/hook/…",
    pattern: /open\.feishu\.cn/,
    configUrl: "https://open.feishu.cn/app",
    steps: [
      "打开飞书 → 进入目标群 → 右上角「…」→「设置」→「群机器人」",
      "点击「添加机器人」→ 选择「自定义机器人」",
      "填写机器人名称和描述 → 点击「添加」",
      "在弹出的「机器人配置」页面复制「Webhook 地址」",
      "回到此页面，将地址粘贴到下方输入框",
    ],
  },
];

onMounted(() => { wizard.setReady(true); });

/** 勾选时自动打开浏览器 */
function onToggle(p: Platform) {
  if (config.platforms[p.id].enabled) {
    openPlatform(p);
  }
}

function openPlatform(p: Platform) {
  tauri.openUrl(p.configUrl).catch(() => {
    window.open(p.configUrl, "_blank");
  });
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
</style>
