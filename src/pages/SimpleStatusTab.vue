<template>
  <div class="status-tab">
    <!-- 服务状态 -->
    <div class="status-card" :class="status.running ? 'running' : 'stopped'">
      <div class="status-header">
        <span class="status-dot"></span>
        <span class="status-text">{{ status.running ? "服务运行中" : "服务已停止" }}</span>
        <span class="version" v-if="status.version">v{{ status.version }}</span>
      </div>
      <div class="status-meta" v-if="status.has_meta">
        <span>端口：{{ status.port }}</span>
        <span>路径：{{ status.install_path }}</span>
      </div>
      <div class="status-meta" v-else>
        <span class="muted">未检测到安装记录</span>
      </div>
    </div>

    <!-- 操作反馈 -->
    <div v-if="errorMsg" class="error-toast">{{ errorMsg }}</div>

    <!-- 快速操作 -->
    <div class="action-bar">
      <button class="btn-primary" v-if="!status.running" @click="startService" :disabled="operating">
        {{ operating ? "操作中…" : "▶ 启动服务" }}
      </button>
      <button class="btn-secondary" v-if="status.running" @click="stopService" :disabled="operating">
        {{ operating ? "操作中…" : "⏹ 停止服务" }}
      </button>
      <button class="btn-secondary" v-if="status.running" @click="restartService" :disabled="operating">
        🔄 重启
      </button>
      <button class="btn-secondary" v-if="status.running" @click="openDashboard">
        🌐 打开官方 Dashboard
      </button>
    </div>

    <!-- 授权状态栏 -->
    <div class="license-bar">
      <template v-if="license.isAuthenticated">
        <span class="plan-badge" :class="license.status.plan">{{ license.planLabel }}</span>
        <span class="license-info">
          到期：{{ license.status.expires_at || '永久' }}
          <span v-if="license.status.in_grace_period" class="grace-warn">（宽限期）</span>
        </span>
        <button class="btn-sm-link" v-if="license.status.plan === 'free'" @click="showPayment = true">升级 Pro</button>
        <button class="btn-sm-link logout" @click="doLogout">退出登录</button>
      </template>
      <template v-else>
        <span class="plan-badge free">免费版</span>
        <div class="license-actions">
          <button class="btn-sm-outline" @click="showActivation = !showActivation">
            🔑 授权码激活
          </button>
          <button class="btn-sm-outline primary" @click="showPayment = true">
            🛒 购买授权
          </button>
        </div>
      </template>
    </div>

    <!-- 授权码激活（内联展开） -->
    <div v-if="showActivation && !license.isAuthenticated" class="activation-card">
      <p class="activation-desc">输入授权码激活付费功能，授权码不限设备。</p>
      <div class="activation-row">
        <input
          v-model="activationCode"
          type="text"
          placeholder="OC-XXXX-XXXX-XXXX"
          class="code-input"
          maxlength="20"
          @keyup.enter="doActivate"
        />
        <button class="btn-primary" @click="doActivate" :disabled="!isCodeValid || activating">
          {{ activating ? "激活中…" : "激活" }}
        </button>
      </div>
      <p v-if="activateError" class="activate-error">{{ activateError }}</p>
      <p v-if="activateOk" class="activate-ok">激活成功！</p>
    </div>

    <!-- AI 配置状态 -->
    <div class="section-card">
      <h3>AI 服务</h3>
      <div v-if="status.ai.configured" class="info-row">
        <span class="dot green"></span>
        <span>{{ status.ai.provider }}</span>
        <span class="muted">{{ status.ai.model }}</span>
      </div>
      <div v-else class="info-row">
        <span class="dot gray"></span>
        <span class="muted">未配置 AI 服务，请在「配置」Tab 中设置</span>
      </div>
    </div>

    <!-- 聊天平台状态 -->
    <div class="section-card">
      <h3>聊天平台</h3>
      <div v-if="platformList.length === 0" class="info-row">
        <span class="dot gray"></span>
        <span class="muted">未配置任何聊天平台</span>
      </div>
      <div v-for="p in platformList" :key="p.name" class="info-row">
        <span class="dot green"></span>
        <span>{{ p.label }}</span>
      </div>
    </div>

    <!-- 购买弹窗 -->
    <PaymentModal
      v-if="showPayment"
      title="升级 Pro 全包"
      plan="pro_all"
      @close="showPayment = false"
      @success="onPaymentSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { tauri } from "@/composables/useTauri";
import { useLicenseStore } from "@/stores/license";
import PaymentModal from "@/components/PaymentModal.vue";

const license = useLicenseStore();
const showPayment = ref(false);
const showActivation = ref(false);
const activationCode = ref("");
const activating = ref(false);
const activateError = ref("");
const activateOk = ref(false);

const isCodeValid = computed(() => {
  const c = activationCode.value.trim().replace(/[-\s]/g, "");
  return c.length >= 12;
});

const status = ref({
  running: false,
  port: 18789,
  has_meta: false,
  version: "",
  install_path: "",
  ai: { configured: false, provider: "", model: "" },
  channels: {} as Record<string, unknown>,
});
const operating = ref(false);
const errorMsg = ref("");

const PLATFORM_LABELS: Record<string, string> = {
  wechat: "企业微信",
  wecom: "企业微信",
  dingtalk: "钉钉",
  feishu: "飞书",
  qq: "QQ",
  whatsapp: "WhatsApp",
  telegram: "Telegram",
  discord: "Discord",
  slack: "Slack",
  line: "LINE",
  imessage: "iMessage",
};

const platformList = computed(() => {
  const ch = status.value.channels;
  if (!ch || typeof ch !== "object") return [];
  return Object.keys(ch)
    .filter((k) => {
      const v = ch[k];
      return v && typeof v === "object" && Object.keys(v as object).length > 0;
    })
    .map((k) => ({ name: k, label: PLATFORM_LABELS[k] || k }));
});

let timer: ReturnType<typeof setInterval> | null = null;

async function refresh() {
  try {
    status.value = await tauri.getGatewayStatus();
  } catch { /* ignore */ }
}

onMounted(async () => {
  await refresh();
  license.loadStatus().catch(() => {});
  timer = setInterval(refresh, 10_000);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});

function showError(msg: string) {
  errorMsg.value = msg;
  setTimeout(() => { errorMsg.value = ""; }, 5000);
}

async function startService() {
  operating.value = true;
  try {
    await tauri.serviceStart();
    await new Promise((r) => setTimeout(r, 3000));
    await refresh();
  } catch (e) { showError(`启动失败: ${e}`); }
  operating.value = false;
}

async function stopService() {
  operating.value = true;
  try {
    await tauri.serviceStop();
    await new Promise((r) => setTimeout(r, 2000));
    await refresh();
  } catch (e) { showError(`停止失败: ${e}`); }
  operating.value = false;
}

async function restartService() {
  operating.value = true;
  try {
    await tauri.serviceStop();
    await new Promise((r) => setTimeout(r, 2000));
    await tauri.serviceStart();
    await new Promise((r) => setTimeout(r, 3000));
    await refresh();
  } catch (e) { showError(`重启失败: ${e}`); }
  operating.value = false;
}

function openDashboard() {
  tauri.openUrl(`http://127.0.0.1:${status.value.port}`);
}

async function doActivate() {
  if (!isCodeValid.value) return;
  activating.value = true;
  activateError.value = "";
  activateOk.value = false;
  try {
    await license.redeemCode(activationCode.value.trim().toUpperCase());
    activateOk.value = true;
    showActivation.value = false;
    activationCode.value = "";
  } catch (e) {
    activateError.value = e instanceof Error ? e.message : String(e);
  } finally {
    activating.value = false;
  }
}

function onPaymentSuccess() {
  showPayment.value = false;
  license.loadStatus();
}

async function doLogout() {
  await license.logout();
}
</script>

<style scoped>
.status-tab { display: flex; flex-direction: column; gap: 16px; }

.status-card {
  padding: 16px 20px; border-radius: var(--radius);
  border: 1px solid var(--color-border);
}
.status-card.running { background: #f0fdf4; border-color: #bbf7d0; }
.status-card.stopped { background: #fef2f2; border-color: #fecaca; }

.status-header { display: flex; align-items: center; gap: 10px; font-size: 15px; font-weight: 600; }
.status-dot {
  width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0;
}
.running .status-dot { background: var(--color-success); }
.stopped .status-dot { background: var(--color-error); }
.version { font-size: 12px; font-weight: 400; color: var(--color-muted); margin-left: auto; }

.status-meta { margin-top: 8px; font-size: 12px; color: var(--color-muted); display: flex; gap: 16px; }

.action-bar { display: flex; gap: 10px; flex-wrap: wrap; }

.section-card {
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 14px 18px;
}
.section-card h3 { font-size: 14px; font-weight: 600; margin-bottom: 10px; }

.info-row { display: flex; align-items: center; gap: 8px; font-size: 13px; padding: 4px 0; }
.dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.dot.green { background: var(--color-success); }
.dot.gray { background: var(--color-muted); }
.muted { color: var(--color-muted); }

.error-toast {
  background: #fef2f2; border: 1px solid #fecaca; color: #991b1b;
  border-radius: var(--radius); padding: 10px 14px; font-size: 13px;
}

/* 授权状态栏 */
.license-bar {
  display: flex; align-items: center; gap: 10px;
  padding: 12px 16px; border-radius: var(--radius);
  background: var(--color-surface); border: 1px solid var(--color-border);
  font-size: 13px; flex-wrap: wrap;
}
.plan-badge {
  padding: 2px 10px; border-radius: 10px;
  font-size: 11px; font-weight: 600;
}
.plan-badge.free { background: #f0fdf4; color: #166534; }
.plan-badge.pro_single, .plan-badge.pro_all { background: #fffbeb; color: #92400e; }
.plan-badge.enterprise { background: #eff6ff; color: #1d4ed8; }
.license-info { flex: 1; color: var(--color-muted); }
.grace-warn { color: var(--color-warning); font-weight: 500; }
.license-actions { display: flex; gap: 8px; margin-left: auto; }
.btn-sm-outline {
  padding: 5px 14px; font-size: 12px; font-weight: 500;
  border: 1px solid var(--color-border); border-radius: var(--radius);
  background: var(--color-surface); cursor: pointer;
  transition: all .15s;
}
.btn-sm-outline:hover { background: var(--color-bg); border-color: var(--color-primary); }
.btn-sm-outline.primary {
  background: var(--color-primary); color: #fff; border-color: var(--color-primary);
}
.btn-sm-outline.primary:hover { opacity: 0.9; }
.btn-sm-link {
  background: none; border: none; color: var(--color-primary);
  font-size: 12px; cursor: pointer; padding: 2px 4px;
}
.btn-sm-link:hover { text-decoration: underline; }
.btn-sm-link.logout { color: var(--color-muted); }

/* 授权码激活区域 */
.activation-card {
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 14px 18px;
}
.activation-desc { font-size: 12px; color: var(--color-muted); margin-bottom: 10px; }
.activation-row { display: flex; gap: 10px; }
.code-input {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius);
  font-size: 14px; font-family: monospace; letter-spacing: 2px;
  text-transform: uppercase;
}
.code-input:focus { outline: none; border-color: var(--color-primary); }
.activate-error { font-size: 12px; color: var(--color-error); margin-top: 8px; }
.activate-ok { font-size: 12px; color: var(--color-success); margin-top: 8px; }
</style>
