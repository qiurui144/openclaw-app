<template>
  <WizardLayout :show-footer="false">
    <div class="finish-page">
      <div class="warn-banner" v-if="wizard.deployStatus !== 'done'">
        ⚠️ 服务尚未运行，请查看日志或手动启动
      </div>

      <div class="hero">
        <div class="success-icon">✅</div>
        <h2>部署完成！</h2>
        <p class="subtitle">OpenClaw 已成功安装到您的系统。</p>
      </div>

      <div class="summary-card">
        <div class="summary-row">
          <span class="s-label">服务地址</span>
          <span class="s-val">http://127.0.0.1:{{ config.servicePort }}</span>
          <button class="copy-btn" @click="copyText('http://127.0.0.1:' + config.servicePort)">复制</button>
        </div>
        <div class="summary-row">
          <span class="s-label">配置文件</span>
          <span class="s-val">~/.openclaw/openclaw.json</span>
        </div>
        <div class="summary-row">
          <span class="s-label">安装路径</span>
          <span class="s-val">{{ config.installPath }}</span>
        </div>
      </div>

      <div class="action-row">
        <button class="btn-primary" @click="openConsole">🌐 打开管理控制台</button>
        <button class="btn-tray" @click="minimizeToTray" title="最小化到系统托盘，OpenClaw 将在后台持续运行">
          🔔 最小化到托盘
        </button>
      </div>

      <!-- 服务实时状态 -->
      <div class="service-status" :class="serviceState">
        <span class="status-dot"></span>
        <span>{{ serviceStateText }}</span>
        <button v-if="serviceState === 'stopped'" class="btn-restart" @click="restartService">▶ 启动服务</button>
      </div>

      <!-- 许可证状态栏 -->
      <div class="license-bar">
        <template v-if="license.isAuthenticated">
          <span class="plan-badge" :class="license.status.plan">{{ license.planLabel }}</span>
          <span class="auth-mode-tag" v-if="license.status.auth_mode === 'code'">授权码</span>
          <span class="auth-mode-tag device" v-else-if="license.status.device_bound">设备绑定</span>
          <span class="license-info">
            到期：{{ license.status.expires_at || '—' }}
            <span v-if="license.status.in_grace_period" class="grace-warn">（宽限期）</span>
          </span>
          <button class="btn-sm-link" @click="upgradeClicked" v-if="license.status.plan === 'free'">升级 Pro</button>
          <button class="btn-sm-link logout" @click="doLogout">退出</button>
        </template>
        <template v-else>
          <span class="plan-badge free">免费版</span>
          <button class="btn-sm-link" @click="showLogin = true">登录 / 激活</button>
          <span class="license-hint">手机号登录或授权码激活</span>
        </template>
      </div>

      <!-- Tab 导航 -->
      <div class="tab-bar">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="tab-btn"
          :class="{ active: activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
          <span v-if="tab.count !== undefined" class="tab-count">{{ tab.count }}</span>
        </button>
      </div>

      <!-- Tab 内容 -->
      <div class="tab-content">
        <!-- 概览 Tab -->
        <div v-if="activeTab === 'overview'" class="overview-tab">
          <div class="next-steps">
            <div class="ns-title">接下来您可以：</div>
            <div class="ns-item">🔐 <span>用设置的管理员密码登录控制台</span></div>
            <div class="ns-item">🔌 <span>在控制台「平台管理」中添加企业微信/钉钉/飞书机器人</span></div>
            <div class="ns-item">🧩 <span>在「Skills 管理」中安装所需的 AI 技能插件</span></div>
            <div class="ns-item">📋 <span>在「日志」中监控机器人运行状态</span></div>
            <div class="ns-item">🔄 <span>通过控制台「系统设置」检查版本更新</span></div>
          </div>

          <div class="skills-section" v-if="updatableSkills.length">
            <h3>可更新的 Skills（{{ updatableSkills.length }}）</h3>
            <div class="skill-list">
              <div class="skill-row" v-for="s in updatableSkills" :key="s.name">
                <span>{{ s.name }}</span>
                <span class="version">{{ s.current_version }} → {{ s.latest_version }}</span>
              </div>
            </div>
            <button class="btn-primary" @click="updateAll" :disabled="updating">
              {{ updating ? "更新中…" : "全部更新" }}
            </button>
          </div>
        </div>

        <!-- 免费 Skills Tab -->
        <div v-if="activeTab === 'free'" class="skills-grid">
          <div v-if="license.loading" class="loading-text">加载中…</div>
          <SkillCard
            v-for="skill in license.freeSkills"
            :key="skill.slug"
            :skill="skill"
            :is-installed="true"
            :has-access="true"
            :operating="false"
          />
          <div v-if="!license.loading && license.freeSkills.length === 0" class="empty-text">
            暂无免费 Skills 数据，请检查网络连接
          </div>
        </div>

        <!-- 付费 Skills Tab -->
        <div v-if="activeTab === 'paid'" class="skills-grid">
          <div v-if="license.loading" class="loading-text">加载中…</div>
          <SkillCard
            v-for="skill in license.paidSkills"
            :key="skill.slug"
            :skill="skill"
            :is-installed="license.installedPaidSlugs.includes(skill.slug)"
            :has-access="license.canAccessSkill(skill.slug)"
            :operating="operatingSlug === skill.slug"
            @install="installSkill"
            @uninstall="uninstallSkill"
            @purchase="purchaseSkill"
          />
          <div v-if="!license.loading && license.paidSkills.length === 0" class="empty-text">
            暂无付费 Skills
          </div>
        </div>

        <!-- 已购 Tab -->
        <div v-if="activeTab === 'purchased'" class="skills-grid">
          <SkillCard
            v-for="skill in purchasedSkills"
            :key="skill.slug"
            :skill="skill"
            :is-installed="license.installedPaidSlugs.includes(skill.slug)"
            :has-access="true"
            :operating="operatingSlug === skill.slug"
            @install="installSkill"
            @uninstall="uninstallSkill"
          />
          <div v-if="purchasedSkills.length === 0" class="empty-text">
            {{ license.isAuthenticated ? '尚未购买任何 Skills' : '请先登录查看已购内容' }}
          </div>
        </div>
      </div>

      <div class="feedback">
        <a href="https://github.com/openclaw/openclaw/issues" target="_blank">🐛 反馈问题</a>
      </div>

      <!-- 弹窗 -->
      <LoginModal v-if="showLogin" @close="showLogin = false" @success="onLoginSuccess" />
      <PaymentModal
        v-if="showPayment"
        :title="paymentTitle"
        :plan="paymentPlan"
        :skill-slug="paymentSlug"
        @close="showPayment = false"
        @success="onPaymentSuccess"
      />
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import WizardLayout from "@/components/WizardLayout.vue";
import SkillCard from "@/components/SkillCard.vue";
import LoginModal from "@/components/LoginModal.vue";
import PaymentModal from "@/components/PaymentModal.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { useLicenseStore, type SkillEntry } from "@/stores/license";
import { tauri, type SkillInfo } from "@/composables/useTauri";

const wizard = useWizardStore();
const config = useConfigStore();
const license = useLicenseStore();

const updatableSkills = ref<SkillInfo[]>([]);
const updating = ref(false);
const activeTab = ref<"overview" | "free" | "paid" | "purchased">("overview");
const operatingSlug = ref<string | null>(null);

// 弹窗状态
const showLogin = ref(false);
const showPayment = ref(false);
const paymentTitle = ref("");
const paymentPlan = ref("");
const paymentSlug = ref<string | undefined>(undefined);

// 服务状态
const serviceState = ref<"running" | "stopped" | "unknown">("unknown");
const serviceStateText = computed(() => ({
  running: "服务运行中 🟢",
  stopped: "服务已停止 🔴",
  unknown: "状态检测中…",
}[serviceState.value]));

// Tab 配置
const tabs = computed(() => [
  { key: "overview" as const, label: "概览", count: undefined },
  { key: "free" as const, label: "免费", count: license.freeSkills.length || undefined },
  { key: "paid" as const, label: "付费", count: license.paidSkills.length || undefined },
  { key: "purchased" as const, label: "已购", count: purchasedSkills.value.length || undefined },
]);

// 已购 Skills = 付费列表中有权访问的
const purchasedSkills = computed(() =>
  license.paidSkills.filter((s) => license.canAccessSkill(s.slug))
);

let statusTimer: ReturnType<typeof setInterval> | null = null;

onMounted(async () => {
  tauri.notifyDeployDone().catch(() => {});

  // 并行加载
  const [skillsResult] = await Promise.allSettled([
    tauri.listSkills(config.installPath),
    license.loadStatus(),
    license.loadSkillIndex(),
    license.loadInstalledPaid(),
  ]);

  // 自动刷新过期的付费 skill 缓存
  license.refreshExpiredSkills(config.installPath).catch(() => {});
  if (skillsResult.status === "fulfilled") {
    updatableSkills.value = skillsResult.value.filter((s) => s.update_available);
  }

  // 轮询服务状态
  const poll = async () => {
    serviceState.value = await tauri.serviceStatus().catch(() => "unknown" as const);
  };
  await poll();
  statusTimer = setInterval(poll, 10_000);
});

onUnmounted(() => {
  if (statusTimer) clearInterval(statusTimer);
});

function openConsole() {
  tauri.openUrl(`http://127.0.0.1:${config.servicePort}`);
}

async function minimizeToTray() {
  await getCurrentWindow().hide();
}

async function restartService() {
  await tauri.serviceStart().catch(() => {});
  setTimeout(async () => {
    serviceState.value = await tauri.serviceStatus().catch(() => "unknown" as const);
  }, 3000);
}

async function updateAll() {
  updating.value = true;
  try {
    await tauri.updateSkills(
      config.installPath,
      updatableSkills.value.map((s) => s.name),
      wizard.clashAccepted ? config.clashSubscriptionUrl : undefined,
    );
    updatableSkills.value = [];
  } finally {
    updating.value = false;
  }
}

async function installSkill(slug: string) {
  operatingSlug.value = slug;
  try {
    await license.installPaidSkill(config.installPath, slug);
  } catch (e) {
    alert(e instanceof Error ? e.message : String(e));
  } finally {
    operatingSlug.value = null;
  }
}

async function uninstallSkill(slug: string) {
  operatingSlug.value = slug;
  try {
    await license.uninstallPaidSkill(slug);
  } finally {
    operatingSlug.value = null;
  }
}

function upgradeClicked() {
  if (!license.isAuthenticated) {
    showLogin.value = true;
    return;
  }
  paymentTitle.value = "升级 Pro 全包";
  paymentPlan.value = "pro_all";
  paymentSlug.value = undefined;
  showPayment.value = true;
}

function purchaseSkill(skill: SkillEntry) {
  if (!license.isAuthenticated) {
    showLogin.value = true;
    return;
  }
  paymentTitle.value = `购买 ${skill.name}`;
  paymentPlan.value = "pro_single";
  paymentSlug.value = skill.slug;
  showPayment.value = true;
}

function onLoginSuccess() {
  license.loadSkillIndex();
  license.loadInstalledPaid();
}

function onPaymentSuccess() {
  showPayment.value = false;
  license.loadSkillIndex();
  license.loadInstalledPaid();
}

async function doLogout() {
  await license.logout();
}

function copyText(text: string) {
  navigator.clipboard.writeText(text).catch(() => {});
}
</script>

<style scoped>
.finish-page { display: flex; flex-direction: column; gap: 16px; }
.warn-banner {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 12px 16px;
  font-size: 13px; color: #92400e;
}
.hero { text-align: center; padding: 8px 0; }
.success-icon { font-size: 48px; }
h2 { font-size: 24px; font-weight: 700; margin-top: 8px; }
.subtitle { color: var(--color-muted); margin-top: 4px; }
.summary-card {
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 16px;
  display: flex; flex-direction: column; gap: 10px;
}
.summary-row { display: flex; align-items: center; gap: 8px; font-size: 13px; }
.s-label { color: var(--color-muted); min-width: 80px; }
.s-val { font-family: monospace; flex: 1; }
.copy-btn { padding: 2px 8px; font-size: 11px; background: var(--color-bg); border: 1px solid var(--color-border); border-radius: 4px; }
.action-row { display: flex; justify-content: center; gap: 10px; flex-wrap: wrap; }
.btn-primary { padding: 10px 24px; font-size: 15px; }
.btn-tray {
  padding: 10px 20px; font-size: 14px;
  border: 1px solid var(--color-border); border-radius: var(--radius);
  background: var(--color-surface); cursor: pointer;
  transition: background .15s;
}
.btn-tray:hover { background: var(--color-bg); }

/* 服务状态 */
.service-status {
  display: flex; align-items: center; gap: 8px;
  padding: 10px 14px; border-radius: var(--radius);
  font-size: 13px; border: 1px solid var(--color-border);
}
.service-status.running { background: #f0fdf4; border-color: #bbf7d0; color: #166534; }
.service-status.stopped { background: #fef2f2; border-color: #fecaca; color: #991b1b; }
.service-status.unknown { background: var(--color-surface); color: var(--color-muted); }
.status-dot {
  width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0;
  background: currentColor;
}
.btn-restart {
  margin-left: auto; padding: 4px 12px; font-size: 12px;
  border: 1px solid currentColor; border-radius: var(--radius);
  background: transparent; cursor: pointer; color: inherit;
}
.btn-restart:hover { background: rgba(0,0,0,.06); }

/* 许可证状态栏 */
.license-bar {
  display: flex; align-items: center; gap: 10px;
  padding: 10px 14px; border-radius: var(--radius);
  background: var(--color-surface); border: 1px solid var(--color-border);
  font-size: 13px;
}
.plan-badge {
  padding: 2px 10px; border-radius: 10px;
  font-size: 11px; font-weight: 600;
}
.plan-badge.free { background: #f0fdf4; color: #166534; }
.plan-badge.pro_single, .plan-badge.pro_all { background: #fffbeb; color: #92400e; }
.plan-badge.enterprise { background: #eff6ff; color: #1d4ed8; }
.auth-mode-tag {
  font-size: 10px; padding: 1px 6px; border-radius: 4px;
  background: #dbeafe; color: #1e40af;
}
.auth-mode-tag.device { background: #fef3c7; color: #92400e; }
.license-info { flex: 1; color: var(--color-muted); }
.grace-warn { color: var(--color-warning); font-weight: 500; }
.license-hint { flex: 1; color: var(--color-muted); }
.btn-sm-link {
  background: none; border: none; color: var(--color-primary);
  font-size: 12px; cursor: pointer; padding: 2px 4px;
}
.btn-sm-link:hover { text-decoration: underline; }
.btn-sm-link.logout { color: var(--color-muted); }

/* Tab 导航 */
.tab-bar {
  display: flex; gap: 0; border-bottom: 2px solid var(--color-border);
}
.tab-btn {
  padding: 8px 16px; font-size: 13px; font-weight: 500;
  background: none; border: none; border-bottom: 2px solid transparent;
  margin-bottom: -2px; cursor: pointer;
  color: var(--color-muted);
  transition: color .15s, border-color .15s;
}
.tab-btn:hover { color: var(--color-text); }
.tab-btn.active { color: var(--color-primary); border-bottom-color: var(--color-primary); }
.tab-count {
  display: inline-block; min-width: 18px; text-align: center;
  padding: 0 5px; margin-left: 4px;
  font-size: 10px; font-weight: 600;
  background: var(--color-bg); border-radius: 8px;
  color: var(--color-muted);
}

/* Tab 内容 */
.tab-content { min-height: 200px; }

/* 概览 */
.next-steps {
  background: var(--color-surface); border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 14px 16px;
  display: flex; flex-direction: column; gap: 8px;
}
.ns-title { font-size: 13px; font-weight: 600; margin-bottom: 4px; }
.ns-item { display: flex; align-items: flex-start; gap: 8px; font-size: 13px; color: #475569; }
.skills-section { margin-top: 12px; }
.skills-section h3 { font-size: 15px; font-weight: 600; margin-bottom: 10px; }
.skill-list { display: flex; flex-direction: column; gap: 6px; margin-bottom: 10px; }
.skill-row { display: flex; justify-content: space-between; font-size: 13px; }
.version { color: var(--color-muted); }

/* Skills 网格 */
.skills-grid {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px; padding-top: 12px;
}
.loading-text, .empty-text {
  grid-column: 1 / -1; text-align: center;
  padding: 32px 0; color: var(--color-muted); font-size: 13px;
}

.feedback { text-align: center; font-size: 13px; }
.feedback a { color: var(--color-muted); }
</style>
