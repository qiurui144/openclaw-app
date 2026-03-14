<template>
  <WizardLayout @next="handleNext" next-label="下一步（可跳过）">
    <div class="ai-token-page">
      <h2>AI 模型接入（可选）</h2>
      <p class="desc">选择 AI 服务商并填入 API Key，OpenClaw 将通过 OpenAI 兼容接口与模型通信。可跳过，安装后在控制台配置。</p>

      <!-- 服务商选择 -->
      <div class="provider-grid">
        <button
          v-for="p in providers"
          :key="p.id"
          class="provider-card"
          :class="{ active: selected === p.id }"
          @click="selectProvider(p)"
        >
          <span class="p-icon">{{ p.icon }}</span>
          <span class="p-name">{{ p.name }}</span>
          <span v-if="p.tag" class="p-tag">{{ p.tag }}</span>
        </button>
      </div>

      <!-- 配置区 -->
      <div v-if="selected" class="config-area">

        <!-- API Key -->
        <div class="field">
          <label>{{ current.keyLabel }}</label>
          <div class="input-row">
            <input
              :type="showKey ? 'text' : 'password'"
              v-model="config.aiApiKey"
              :placeholder="current.keyPlaceholder"
              class="key-input"
              @input="onKeyInput"
              autocomplete="off"
            />
            <button class="eye-btn" @click="showKey = !showKey">{{ showKey ? "🙈" : "👁️" }}</button>
            <button class="open-btn" @click="openDocs(current)" title="获取 API Key">获取 ↗</button>
          </div>
        </div>

        <!-- Base URL（自定义或需要显示的情况） -->
        <div class="field" v-if="selected === 'custom' || showAdvanced">
          <label>API Base URL</label>
          <input type="url" v-model="config.aiBaseUrl" :placeholder="current.baseUrl || 'https://...'" />
        </div>

        <!-- 模型 -->
        <div class="field">
          <label>模型名称</label>
          <div v-if="current.models.length > 0" class="model-row">
            <select v-model="config.aiModel" class="model-select">
              <option v-for="m in current.models" :key="m" :value="m">{{ m }}</option>
              <option value="__custom__">自定义…</option>
            </select>
            <input
              v-if="config.aiModel === '__custom__'"
              type="text"
              v-model="customModel"
              placeholder="输入模型名称"
              class="custom-model-input"
            />
          </div>
          <input
            v-else
            type="text"
            v-model="config.aiModel"
            :placeholder="current.modelHint || '输入模型名称'"
          />
          <span class="hint" v-if="current.modelHint && current.models.length === 0">{{ current.modelHint }}</span>
        </div>

        <!-- 高级选项切换 -->
        <button
          v-if="selected !== 'custom'"
          class="advanced-toggle"
          @click="showAdvanced = !showAdvanced"
        >
          {{ showAdvanced ? "▲ 收起高级选项" : "▼ 修改 Base URL（高级）" }}
        </button>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();

interface Provider {
  id: string;
  icon: string;
  name: string;
  tag?: string;
  baseUrl: string;
  models: string[];
  keyLabel: string;
  keyPlaceholder: string;
  docsUrl: string;
  modelHint?: string;
}

const providers: Provider[] = [
  {
    id: "qwen",
    icon: "🔮",
    name: "通义千问",
    tag: "推荐",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    models: ["qwen-max", "qwen-plus", "qwen-turbo", "qwen-long"],
    keyLabel: "DashScope API Key",
    keyPlaceholder: "sk-...",
    docsUrl: "https://dashscope.console.aliyun.com/apiKey",
  },
  {
    id: "deepseek",
    icon: "🌊",
    name: "DeepSeek",
    tag: "推荐",
    baseUrl: "https://api.deepseek.com/v1",
    models: ["deepseek-chat", "deepseek-reasoner"],
    keyLabel: "API Key",
    keyPlaceholder: "sk-...",
    docsUrl: "https://platform.deepseek.com/api_keys",
  },
  {
    id: "doubao",
    icon: "🫘",
    name: "豆包",
    baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
    models: [],
    keyLabel: "API Key（火山引擎）",
    keyPlaceholder: "...",
    docsUrl: "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    modelHint: "填写火山引擎「模型推理」创建的接入点 ID",
  },
  {
    id: "glm",
    icon: "🧠",
    name: "智谱AI",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    models: ["glm-4", "glm-4-flash", "glm-4-0520"],
    keyLabel: "API Key",
    keyPlaceholder: "...",
    docsUrl: "https://open.bigmodel.cn/usercenter/apikeys",
  },
  {
    id: "moonshot",
    icon: "🌙",
    name: "Kimi",
    baseUrl: "https://api.moonshot.cn/v1",
    models: ["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"],
    keyLabel: "API Key",
    keyPlaceholder: "sk-...",
    docsUrl: "https://platform.moonshot.cn/console/api-keys",
  },
  {
    id: "openai",
    icon: "🤖",
    name: "OpenAI",
    baseUrl: "https://api.openai.com/v1",
    models: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"],
    keyLabel: "API Key",
    keyPlaceholder: "sk-...",
    docsUrl: "https://platform.openai.com/api-keys",
  },
  {
    id: "custom",
    icon: "⚙️",
    name: "自定义",
    baseUrl: "",
    models: [],
    keyLabel: "API Key",
    keyPlaceholder: "...",
    docsUrl: "",
    modelHint: "填写兼容 OpenAI 接口的任意模型名",
  },
];

const selected = ref(config.aiProvider || "");
const showKey = ref(false);
const showAdvanced = ref(false);
const customModel = ref("");

const current = computed<Provider>(
  () => providers.find((p) => p.id === selected.value) ?? providers[providers.length - 1]
);

// 选择服务商时重置配置
function selectProvider(p: Provider) {
  selected.value = p.id;
  config.aiProvider = p.id;
  config.aiBaseUrl = p.baseUrl;
  config.aiModel = p.models[0] ?? "";
  showAdvanced.value = false;
}

// 同步自定义模型输入
watch(customModel, (v) => {
  if (config.aiModel === "__custom__") config.aiModel = v;
});
watch(() => config.aiModel, (v) => {
  if (v !== "__custom__" && v) customModel.value = "";
});

function onKeyInput() {
  config.aiApiKey = config.aiApiKey.trim();
}

function openDocs(p: Provider) {
  if (p.docsUrl) {
    tauri.openUrl(p.docsUrl).catch(() => { window.open(p.docsUrl, "_blank"); });
  }
}

// 此步骤可跳过，始终可继续
wizard.setReady(true);

function handleNext() { next(); }
</script>

<style scoped>
.ai-token-page { display: flex; flex-direction: column; gap: 18px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { font-size: 13px; color: var(--color-muted); margin-top: -10px; }

/* 服务商选择网格 */
.provider-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}
.provider-card {
  display: flex; flex-direction: column; align-items: center;
  gap: 4px; padding: 12px 8px;
  border: 2px solid var(--color-border); border-radius: var(--radius);
  background: var(--color-surface); cursor: pointer;
  transition: border-color .15s, background .15s;
  position: relative;
}
.provider-card:hover { border-color: var(--color-primary); }
.provider-card.active { border-color: var(--color-primary); background: #eff6ff; }
.p-icon { font-size: 22px; }
.p-name { font-size: 12px; font-weight: 600; text-align: center; }
.p-tag {
  position: absolute; top: 4px; right: 4px;
  background: var(--color-primary); color: #fff;
  font-size: 9px; padding: 1px 5px; border-radius: 8px; font-weight: 600;
}

/* 配置区 */
.config-area { display: flex; flex-direction: column; gap: 14px; }

.field { display: flex; flex-direction: column; gap: 6px; }
label { font-size: 13px; font-weight: 500; }

.input-row { display: flex; gap: 6px; align-items: center; }
.key-input {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 13px;
  font-family: monospace;
}
.eye-btn, .open-btn {
  padding: 7px 10px; font-size: 13px; border-radius: var(--radius);
  border: 1px solid var(--color-border); background: var(--color-surface); cursor: pointer;
  white-space: nowrap; flex-shrink: 0;
}
.open-btn { color: var(--color-primary); border-color: var(--color-primary); }
.open-btn:hover { background: var(--color-primary); color: #fff; }

input[type="url"], input[type="text"] {
  padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; width: 100%; box-sizing: border-box;
}

.model-row { display: flex; gap: 8px; align-items: center; }
.model-select {
  flex: 1; padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; background: var(--color-bg);
}
.custom-model-input {
  flex: 1; padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px;
}

.hint { font-size: 12px; color: var(--color-muted); }

.advanced-toggle {
  align-self: flex-start; background: none; border: none;
  font-size: 12px; color: var(--color-muted); cursor: pointer; padding: 0;
}
.advanced-toggle:hover { color: var(--color-primary); }
</style>
