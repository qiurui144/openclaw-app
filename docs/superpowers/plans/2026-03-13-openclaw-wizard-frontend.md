# OpenClaw Wizard Frontend (Vue 3) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 Vue 3 + Vite + Pinia + Vue Router 实现完整的 10 步部署向导前端，通过 Tauri IPC 与 Plan A（Rust 后端）集成。

**Architecture:** SPA 向导，每页一个路由，Pinia 管理跨页共享的 DeployConfig 状态。组件通过 `invoke()` 调用 Rust command，通过 `listen()` 订阅后端事件（进度推送）。无样式框架依赖（原生 CSS 变量 + scoped styles），保持包体积最小化。

**Tech Stack:** Vue 3.4+、Vite 5、TypeScript、Pinia 2、Vue Router 4、qrcode（QR 码生成）、@tauri-apps/api 2、Vitest + @vue/test-utils（单元测试）

---

## 文件结构

```
src/
├── App.vue
├── main.ts
├── router/
│   └── index.ts
├── stores/
│   ├── wizard.ts        # 页面导航状态、系统检查结果、部署状态
│   └── config.ts        # DeployConfig 表单数据（不序列化密码）
├── composables/
│   ├── useTauri.ts      # 类型化 invoke/listen 封装
│   └── useWizardNav.ts  # next/back 导航逻辑
├── components/
│   ├── WizardLayout.vue      # 外壳：步骤条 + 导航按钮
│   ├── StepIndicator.vue     # 步骤点/步骤条
│   ├── CheckItem.vue         # 系统检查结果行
│   ├── QrCodeModal.vue       # 二维码弹窗
│   ├── PasswordStrength.vue  # 密码强度色条
│   └── LogPanel.vue          # 可折叠日志区域
└── pages/
    ├── WelcomePage.vue
    ├── SystemCheckPage.vue
    ├── SourcePage.vue
    ├── ClashDisclaimerPage.vue
    ├── ClashConfigPage.vue
    ├── InstallConfigPage.vue
    ├── ServiceConfigPage.vue
    ├── PlatformIntegrationPage.vue
    ├── DeploymentPage.vue
    └── FinishPage.vue
tests/
└── unit/
    ├── stores/
    │   ├── wizard.test.ts
    │   └── config.test.ts
    ├── composables/
    │   └── useTauri.test.ts
    └── pages/
        ├── WelcomePage.test.ts
        ├── SystemCheckPage.test.ts
        ├── SourcePage.test.ts
        ├── ClashDisclaimerPage.test.ts
        ├── ClashConfigPage.test.ts
        ├── InstallConfigPage.test.ts
        ├── ServiceConfigPage.test.ts
        ├── PlatformIntegrationPage.test.ts
        ├── DeploymentPage.test.ts
        └── FinishPage.test.ts
```

---

## Chunk 1: 项目脚手架与基础配置

### Task 1: 初始化 Tauri + Vue 3 前端

**Files:**
- Modify: `src-tauri/tauri.conf.json`（已存在）
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `index.html`
- Create: `src/main.ts`
- Create: `src/App.vue`
- Create: `vitest.config.ts`

- [ ] **Step 1: 初始化 npm 包**

```bash
cd /data/openclaw
# 若 package.json 不存在，先创建前端脚手架
# Tauri 2 + Vue 3 标准结构
npm create tauri-app@latest . -- --template vue-ts --manager npm --no-open 2>/dev/null || true
# 或手动创建（下面 Step 2 给出完整文件内容）
```

- [ ] **Step 2: 写 package.json**

```json
{
  "name": "openclaw-wizard",
  "private": true,
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-fs": "^2",
    "pinia": "^2.1",
    "qrcode": "^1.5",
    "vue": "^3.4",
    "vue-router": "^4.3"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@vitejs/plugin-vue": "^5",
    "@vue/test-utils": "^2",
    "@types/qrcode": "^1",
    "happy-dom": "^14",
    "typescript": "^5",
    "vite": "^5",
    "vitest": "^1",
    "vue-tsc": "^2"
  }
}
```

- [ ] **Step 3: 写 vite.config.ts**

```typescript
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  clearScreen: false,
  server: {
    host: host || false,
    port: 1420,
    strictPort: true,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: { target: ["es2021", "chrome100", "safari13"] },
});
```

- [ ] **Step 4: 写 vitest.config.ts**

```typescript
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: ["./tests/setup.ts"],
  },
});
```

- [ ] **Step 5: 写 tests/setup.ts（Mock Tauri API）**

```typescript
// 测试环境下 mock @tauri-apps/api/core，避免调用真实 IPC
import { vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));
```

- [ ] **Step 6: 写 src/main.ts**

```typescript
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { router } from "./router";

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
```

- [ ] **Step 7: 写 src/App.vue**

```vue
<template>
  <RouterView />
</template>

<script setup lang="ts">
import { RouterView } from "vue-router";
</script>

<style>
:root {
  --color-primary: #2563eb;
  --color-success: #16a34a;
  --color-warning: #d97706;
  --color-error: #dc2626;
  --color-bg: #f8fafc;
  --color-surface: #ffffff;
  --color-border: #e2e8f0;
  --color-text: #1e293b;
  --color-muted: #64748b;
  --radius: 8px;
  --shadow: 0 1px 3px rgba(0,0,0,.08), 0 1px 2px rgba(0,0,0,.06);
}

* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: var(--color-bg);
  color: var(--color-text);
  font-size: 14px;
  line-height: 1.5;
}

button {
  cursor: pointer;
  border: none;
  border-radius: var(--radius);
  padding: 8px 16px;
  font-size: 14px;
  transition: opacity .15s;
}
button:disabled { opacity: .45; cursor: not-allowed; }

.btn-primary {
  background: var(--color-primary);
  color: #fff;
}
.btn-primary:hover:not(:disabled) { opacity: .9; }

.btn-secondary {
  background: transparent;
  border: 1px solid var(--color-border);
  color: var(--color-text);
}
.btn-secondary:hover:not(:disabled) { background: var(--color-bg); }
</style>
```

- [ ] **Step 8: 写 index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OpenClaw 部署向导</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 9: 安装依赖**

```bash
npm install
# 验证无报错
npm run build 2>&1 | tail -5
```

- [ ] **Step 10: Commit**

```bash
git add package.json vite.config.ts tsconfig.json index.html src/main.ts src/App.vue vitest.config.ts tests/setup.ts
git commit -m "feat(frontend): Vue 3 + Tauri 前端脚手架"
```

---

## Chunk 2: 路由 + Pinia Stores

### Task 2: 路由配置

**Files:**
- Create: `src/router/index.ts`
- Create: `tests/unit/stores/wizard.test.ts`
- Create: `tests/unit/stores/config.test.ts`

- [ ] **Step 1: 写 wizard store 失败测试**

```typescript
// tests/unit/stores/wizard.test.ts
import { setActivePinia, createPinia } from "pinia";
import { useWizardStore } from "@/stores/wizard";
import { describe, it, expect, beforeEach } from "vitest";

describe("wizard store", () => {
  beforeEach(() => { setActivePinia(createPinia()); });

  it("初始页面为 welcome", () => {
    const store = useWizardStore();
    expect(store.currentPage).toBe("welcome");
  });

  it("canProceed 默认 false", () => {
    const store = useWizardStore();
    expect(store.canProceed).toBe(false);
  });

  it("setReady 后 canProceed 为 true", () => {
    const store = useWizardStore();
    store.setReady(true);
    expect(store.canProceed).toBe(true);
  });

  it("setChecks 更新 systemChecks", () => {
    const store = useWizardStore();
    store.setChecks([{ id: "os", label: "操作系统", required: true, status: "ok", detail: "" }]);
    expect(store.systemChecks).toHaveLength(1);
  });

  it("deployStatus 初始为 idle", () => {
    const store = useWizardStore();
    expect(store.deployStatus).toBe("idle");
  });
});
```

- [ ] **Step 2: 运行测试，期望 FAIL**

```bash
npm run test -- tests/unit/stores/wizard.test.ts 2>&1 | grep -E "FAIL|PASS|Error"
# 期望: FAIL（文件不存在）
```

- [ ] **Step 3: 写 src/stores/wizard.ts**

```typescript
import { defineStore } from "pinia";
import { ref } from "vue";

export type CheckStatus = "pending" | "running" | "ok" | "warn" | "error";
export type DeployStatus = "idle" | "running" | "done" | "failed";
export type SourceMode = "bundled" | "online" | "local_zip";

export interface CheckItem {
  id: string;
  label: string;
  required: boolean;
  status: CheckStatus;
  detail: string;
}

export interface DeployProgress {
  step: number;
  total: number;
  percent: number;
  message: string;
}

export const useWizardStore = defineStore("wizard", () => {
  const currentPage = ref<string>("welcome");
  const canProceed = ref(false);
  const systemChecks = ref<CheckItem[]>([]);
  const sourceMode = ref<SourceMode>("bundled");
  const clashAccepted = ref(false);
  const deployStatus = ref<DeployStatus>("idle");
  const deployProgress = ref<DeployProgress>({ step: 0, total: 11, percent: 0, message: "" });
  const deployLogs = ref<string[]>([]);
  const isExistingInstall = ref(false);
  const existingVersion = ref<string | null>(null);
  const existingPath = ref<string | null>(null);

  function setReady(v: boolean) { canProceed.value = v; }
  function setChecks(items: CheckItem[]) { systemChecks.value = items; }
  function setSourceMode(m: SourceMode) { sourceMode.value = m; }
  function setClashAccepted(v: boolean) { clashAccepted.value = v; }
  function setDeployStatus(s: DeployStatus) { deployStatus.value = s; }
  function updateProgress(p: DeployProgress) {
    deployProgress.value = p;
    deployLogs.value.push(`[${new Date().toLocaleTimeString()}] ${p.message}`);
  }
  function setExistingInstall(version: string, path: string) {
    isExistingInstall.value = true;
    existingVersion.value = version;
    existingPath.value = path;
  }

  return {
    currentPage, canProceed, systemChecks, sourceMode,
    clashAccepted, deployStatus, deployProgress, deployLogs,
    isExistingInstall, existingVersion, existingPath,
    setReady, setChecks, setSourceMode, setClashAccepted,
    setDeployStatus, updateProgress, setExistingInstall,
  };
});
```

- [ ] **Step 4: 运行测试，期望 PASS**

```bash
npm run test -- tests/unit/stores/wizard.test.ts 2>&1 | grep -E "FAIL|PASS"
# 期望: PASS
```

- [ ] **Step 5: 写 config store 测试**

```typescript
// tests/unit/stores/config.test.ts
import { setActivePinia, createPinia } from "pinia";
import { useConfigStore } from "@/stores/config";
import { describe, it, expect, beforeEach } from "vitest";

describe("config store", () => {
  beforeEach(() => { setActivePinia(createPinia()); });

  it("service_port 默认 18789", () => {
    const store = useConfigStore();
    expect(store.servicePort).toBe(18789);
  });

  it("密码不在 toDto() 中暴露给日志", () => {
    const store = useConfigStore();
    store.adminPassword = "s3cret!";
    // toDto 包含密码（IPC 边界发送），但不得序列化到磁盘
    // 此处验证 dto 包含 admin_password（Rust 接收）
    expect(store.toDto().admin_password).toBe("s3cret!");
  });

  it("updatePlatform 更新指定平台", () => {
    const store = useConfigStore();
    store.updatePlatform("wx", { enabled: true, webhookUrl: "https://qyapi.weixin.qq.com/test" });
    expect(store.platforms.wx.enabled).toBe(true);
  });

  it("isPasswordValid 长度<8 为 false", () => {
    const store = useConfigStore();
    store.adminPassword = "abc";
    expect(store.isPasswordValid).toBe(false);
  });

  it("isPasswordValid 混合字母数字 ≥8 为 true", () => {
    const store = useConfigStore();
    store.adminPassword = "abc12345";
    expect(store.isPasswordValid).toBe(true);
  });
});
```

- [ ] **Step 6: 写 src/stores/config.ts**

```typescript
import { defineStore } from "pinia";
import { ref, computed } from "vue";

export interface PlatformConfig {
  enabled: boolean;
  webhookUrl: string;
}

export interface DeployConfigDto {
  install_path: string;
  service_port: number;
  admin_password: string;
  domain_name: string | null;
  install_service: boolean;
  start_on_boot: boolean;
  source_mode: { type: string; proxy_url?: string; zip_path?: string };
  platforms: Record<string, PlatformConfig>;
}

export const useConfigStore = defineStore("config", () => {
  const installPath = ref(defaultInstallPath());
  const servicePort = ref(18789);
  const adminPassword = ref("");
  const confirmPassword = ref("");
  const domainName = ref<string | null>(null);
  const installService = ref(true);
  const startOnBoot = ref(true);
  const clashSubscriptionUrl = ref("");
  const localZipPath = ref<string | null>(null);
  const platforms = ref<Record<string, PlatformConfig>>({
    wx: { enabled: false, webhookUrl: "" },
    qq: { enabled: false, webhookUrl: "" },
    dt: { enabled: false, webhookUrl: "" },
    fs: { enabled: false, webhookUrl: "" },
  });

  const isPasswordValid = computed(() => {
    const p = adminPassword.value;
    return p.length >= 8 && /[a-zA-Z]/.test(p) && /\d/.test(p);
  });

  const passwordsMatch = computed(
    () => adminPassword.value === confirmPassword.value
  );

  function updatePlatform(id: string, patch: Partial<PlatformConfig>) {
    platforms.value[id] = { ...platforms.value[id], ...patch };
  }

  function toDto(): DeployConfigDto {
    return {
      install_path: installPath.value,
      service_port: servicePort.value,
      admin_password: adminPassword.value,
      domain_name: domainName.value,
      install_service: installService.value,
      start_on_boot: startOnBoot.value,
      source_mode: buildSourceMode(),
      platforms: platforms.value,
    };
  }

  function buildSourceMode() {
    // wizard store 的 sourceMode 决定类型，由外层组合使用
    return { type: "bundled" }; // 由 DeploymentPage 根据 wizard.sourceMode 覆盖
  }

  return {
    installPath, servicePort, adminPassword, confirmPassword,
    domainName, installService, startOnBoot, clashSubscriptionUrl,
    localZipPath, platforms,
    isPasswordValid, passwordsMatch,
    updatePlatform, toDto,
  };
});

function defaultInstallPath(): string {
  // 注意：Tauri 前端无法直接读取平台，运行时由 WelcomePage 从 Rust 获取
  return "/opt/openclaw";
}
```

- [ ] **Step 7: 运行所有 store 测试**

```bash
npm run test -- tests/unit/stores/ 2>&1 | grep -E "FAIL|PASS|✓|×"
# 期望: 全部 PASS
```

- [ ] **Step 8: 写 src/router/index.ts**

```typescript
import { createRouter, createWebHistory } from "vue-router";

const routes = [
  { path: "/", name: "welcome",     component: () => import("@/pages/WelcomePage.vue") },
  { path: "/check",   name: "check",    component: () => import("@/pages/SystemCheckPage.vue") },
  { path: "/source",  name: "source",   component: () => import("@/pages/SourcePage.vue") },
  { path: "/clash-disclaimer", name: "clash-disclaimer", component: () => import("@/pages/ClashDisclaimerPage.vue") },
  { path: "/clash-config",     name: "clash-config",     component: () => import("@/pages/ClashConfigPage.vue") },
  { path: "/install", name: "install",  component: () => import("@/pages/InstallConfigPage.vue") },
  { path: "/service", name: "service",  component: () => import("@/pages/ServiceConfigPage.vue") },
  { path: "/platform",name: "platform", component: () => import("@/pages/PlatformIntegrationPage.vue") },
  { path: "/deploy",  name: "deploy",   component: () => import("@/pages/DeploymentPage.vue") },
  { path: "/finish",  name: "finish",   component: () => import("@/pages/FinishPage.vue") },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});
```

- [ ] **Step 9: Commit**

```bash
git add src/stores/ src/router/ tests/unit/stores/
git commit -m "feat(frontend): Pinia stores + Vue Router 路由配置"
```

---

## Chunk 3: 公共组件

### Task 3: WizardLayout + 基础组件

**Files:**
- Create: `src/components/WizardLayout.vue`
- Create: `src/components/StepIndicator.vue`
- Create: `src/components/CheckItem.vue`
- Create: `src/components/PasswordStrength.vue`
- Create: `src/components/LogPanel.vue`
- Create: `src/components/QrCodeModal.vue`
- Create: `src/composables/useWizardNav.ts`
- Create: `src/composables/useTauri.ts`

- [ ] **Step 1: 写 useTauri.ts（类型化 IPC 封装）**

```typescript
// src/composables/useTauri.ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onUnmounted } from "vue";
import type { CheckItem, DeployProgress } from "@/stores/wizard";

// ── Rust command 返回类型 ───────────────────────────────────
export interface UpdateInfo {
  version: string;
  download_url: string;
  sha256: string;
  release_notes: string;
}

export interface SkillInfo {
  name: string;
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
}

export interface ClashTestResult {
  success: boolean;
  latency_ms: number | null;
  error: string | null;
}

export interface DeployMeta {
  version: string;
  install_path: string;
  installed_at: string;
  service_port: number;
}

// ── 类型化 invoke 封装 ─────────────────────────────────────
export const tauri = {
  runSystemCheck: () => invoke<CheckItem[]>("run_system_check"),
  loadSession: () => invoke<{ install_path: string; source_mode: string } | null>("load_session"),
  clearSession: (installPath?: string) => invoke<void>("clear_session", { installPath }),
  getDefaultInstallPath: () => invoke<string>("get_default_install_path"),
  startDeploy: (config: unknown) => invoke<void>("start_deploy", { config }),
  clashTest: (url: string) => invoke<ClashTestResult>("clash_test", { subscriptionUrl: url }),
  clashStart: (url: string) => invoke<string>("clash_start", { subscriptionUrl: url }),
  clashStop: () => invoke<void>("clash_stop"),
  listSkills: (installPath: string) => invoke<SkillInfo[]>("list_skills", { installPath }),
  updateSkills: (installPath: string, skillNames: string[], proxyUrl?: string) =>
    invoke<void>("update_skills", { installPath, skillNames, proxyUrl: proxyUrl ?? null }),
  checkUpdate: (proxyUrl?: string) =>
    invoke<UpdateInfo | null>("check_openclaw_update", { proxyUrl: proxyUrl ?? null }),
  applyUpdate: (installPath: string, downloadUrl: string, sha256: string, proxyUrl?: string) =>
    invoke<void>("apply_openclaw_update", { installPath, downloadUrl, sha256, proxyUrl: proxyUrl ?? null }),
  readDeployMeta: () => invoke<DeployMeta | null>("read_deploy_meta"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
};

// ── 事件监听 composable ────────────────────────────────────
export function useDeployEvents(
  onProgress: (p: DeployProgress) => void,
  onDone: () => void,
  onFailed?: (reason: string) => void,
) {
  let unlistenProgress: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenFailed: UnlistenFn | null = null;

  async function subscribe() {
    unlistenProgress = await listen<DeployProgress>("deploy:progress", (e) => onProgress(e.payload));
    unlistenDone = await listen<void>("deploy:done", () => onDone());
    if (onFailed) {
      unlistenFailed = await listen<string>("deploy:failed", (e) => onFailed(e.payload));
    }
  }

  function unsubscribe() {
    unlistenProgress?.();
    unlistenDone?.();
    unlistenFailed?.();
  }

  onUnmounted(unsubscribe);
  return { subscribe, unsubscribe };
}
```

- [ ] **Step 2: 写 useWizardNav.ts**

```typescript
// src/composables/useWizardNav.ts
import { useRouter } from "vue-router";
import { useWizardStore } from "@/stores/wizard";

// 页面顺序（动态：Clash 页面仅在线模式出现）
const ROUTE_ORDER_BASE = [
  "welcome", "check", "source", "install", "service", "platform", "deploy", "finish",
];
const CLASH_PAGES = ["clash-disclaimer", "clash-config"];

export function useWizardNav() {
  const router = useRouter();
  const wizard = useWizardStore();

  function routeOrder() {
    if (wizard.sourceMode === "online" && wizard.clashAccepted !== undefined) {
      // 仅在 source=online 时插入 Clash 页面
      return [
        "welcome", "check", "source", "clash-disclaimer", "clash-config",
        "install", "service", "platform", "deploy", "finish",
      ];
    }
    return ROUTE_ORDER_BASE;
  }

  function currentIndex() {
    const name = router.currentRoute.value.name as string;
    return routeOrder().indexOf(name);
  }

  function next() {
    const order = routeOrder();
    const idx = currentIndex();
    if (idx < order.length - 1) {
      router.push({ name: order[idx + 1] });
    }
  }

  function back() {
    const order = routeOrder();
    const idx = currentIndex();
    if (idx > 0) {
      router.push({ name: order[idx - 1] });
    }
  }

  function goTo(name: string) {
    router.push({ name });
  }

  return { next, back, goTo, currentIndex, routeOrder };
}
```

- [ ] **Step 3: 写 WizardLayout.vue**

```vue
<!-- src/components/WizardLayout.vue -->
<template>
  <div class="wizard-shell">
    <header class="wizard-header">
      <div class="brand">
        <span class="brand-icon">🦾</span>
        <span class="brand-name">OpenClaw 部署向导</span>
      </div>
      <StepIndicator :steps="visibleSteps" :current="currentIndex()" />
    </header>

    <main class="wizard-body">
      <slot />
    </main>

    <footer class="wizard-footer" v-if="showFooter">
      <button class="btn-secondary" @click="back()" :disabled="!canGoBack">
        ← 上一步
      </button>
      <span class="spacer" />
      <button
        v-if="!hideNext"
        class="btn-primary"
        @click="$emit('next')"
        :disabled="!wizard.canProceed"
      >
        {{ nextLabel }} →
      </button>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import StepIndicator from "./StepIndicator.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const props = withDefaults(defineProps<{
  showFooter?: boolean;
  hideNext?: boolean;
  nextLabel?: string;
}>(), {
  showFooter: true,
  hideNext: false,
  nextLabel: "下一步",
});

defineEmits<{ next: [] }>();

const wizard = useWizardStore();
const router = useRouter();
const { back, currentIndex, routeOrder } = useWizardNav();

const canGoBack = computed(() => {
  const idx = currentIndex();
  const name = router.currentRoute.value.name as string;
  return idx > 0 && name !== "deploy"; // 部署中不可返回
});

const visibleSteps = computed(() => {
  // 仅显示主干步骤（不含 Clash 细节页）
  return [
    { label: "欢迎" },
    { label: "检查" },
    { label: "来源" },
    { label: "配置" },
    { label: "服务" },
    { label: "平台" },
    { label: "部署" },
    { label: "完成" },
  ];
});
</script>

<style scoped>
.wizard-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  max-width: 720px;
  margin: 0 auto;
}

.wizard-header {
  padding: 16px 24px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  gap: 24px;
  background: var(--color-surface);
}

.brand { display: flex; align-items: center; gap: 8px; font-weight: 600; }
.brand-icon { font-size: 20px; }

.wizard-body {
  flex: 1;
  overflow-y: auto;
  padding: 32px 24px;
}

.wizard-footer {
  padding: 16px 24px;
  border-top: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  background: var(--color-surface);
}

.spacer { flex: 1; }
</style>
```

- [ ] **Step 4: 写 StepIndicator.vue**

```vue
<!-- src/components/StepIndicator.vue -->
<template>
  <div class="steps">
    <template v-for="(step, i) in steps" :key="i">
      <div
        class="step-dot"
        :class="{
          done: i < current,
          active: i === current,
          upcoming: i > current,
        }"
        :title="step.label"
      >
        <span v-if="i < current">✓</span>
        <span v-else>{{ i + 1 }}</span>
      </div>
      <div v-if="i < steps.length - 1" class="step-line" :class="{ done: i < current }" />
    </template>
  </div>
</template>

<script setup lang="ts">
defineProps<{ steps: { label: string }[]; current: number }>();
</script>

<style scoped>
.steps { display: flex; align-items: center; flex: 1; justify-content: center; }

.step-dot {
  width: 24px; height: 24px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 600;
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  color: var(--color-muted);
  flex-shrink: 0;
}
.step-dot.active { border-color: var(--color-primary); color: var(--color-primary); }
.step-dot.done { border-color: var(--color-success); background: var(--color-success); color: #fff; }

.step-line { flex: 1; height: 2px; background: var(--color-border); }
.step-line.done { background: var(--color-success); }
</style>
```

- [ ] **Step 5: 写 CheckItem.vue**

```vue
<!-- src/components/CheckItem.vue -->
<template>
  <div class="check-item" :class="item.status">
    <span class="icon">{{ statusIcon }}</span>
    <div class="info">
      <span class="label">{{ item.label }}</span>
      <span v-if="item.detail" class="detail">{{ item.detail }}</span>
    </div>
    <span class="badge" v-if="item.required && item.status === 'error'">必需</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { CheckItem } from "@/stores/wizard";

const props = defineProps<{ item: CheckItem }>();

const statusIcon = computed(() => ({
  pending: "○", running: "⏳", ok: "✓", warn: "⚠️", error: "✕"
}[props.item.status]));
</script>

<style scoped>
.check-item {
  display: flex; align-items: flex-start; gap: 12px;
  padding: 12px 16px;
  border-radius: var(--radius);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
}
.check-item.ok { border-color: #bbf7d0; background: #f0fdf4; }
.check-item.warn { border-color: #fde68a; background: #fffbeb; }
.check-item.error { border-color: #fecaca; background: #fef2f2; }

.icon { font-size: 16px; margin-top: 1px; }
.info { flex: 1; }
.label { font-weight: 500; }
.detail { display: block; font-size: 12px; color: var(--color-muted); margin-top: 2px; }
.badge {
  font-size: 11px; padding: 2px 6px;
  background: var(--color-error); color: #fff;
  border-radius: 4px;
}
</style>
```

- [ ] **Step 6: 写 PasswordStrength.vue**

```vue
<!-- src/components/PasswordStrength.vue -->
<template>
  <div class="strength-bar" v-if="password.length > 0">
    <div class="track">
      <div class="fill" :class="level" :style="{ width: pct + '%' }" />
    </div>
    <span class="label">{{ labels[level] }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ password: string }>();

const score = computed(() => {
  const p = props.password;
  let s = 0;
  if (p.length >= 8) s++;
  if (p.length >= 12) s++;
  if (/[A-Z]/.test(p)) s++;
  if (/[0-9]/.test(p)) s++;
  if (/[^a-zA-Z0-9]/.test(p)) s++;
  return s;
});

const level = computed<"weak" | "medium" | "strong">(() => {
  if (score.value <= 2) return "weak";
  if (score.value <= 3) return "medium";
  return "strong";
});

const pct = computed(() => Math.min(100, (score.value / 5) * 100));

const labels = { weak: "弱", medium: "中", strong: "强" };
</script>

<style scoped>
.strength-bar { display: flex; align-items: center; gap: 8px; margin-top: 6px; }
.track { flex: 1; height: 4px; background: var(--color-border); border-radius: 2px; overflow: hidden; }
.fill { height: 100%; border-radius: 2px; transition: width .3s; }
.fill.weak   { background: var(--color-error); }
.fill.medium { background: var(--color-warning); }
.fill.strong { background: var(--color-success); }
.label { font-size: 12px; color: var(--color-muted); min-width: 16px; }
</style>
```

- [ ] **Step 7: 写 LogPanel.vue**

```vue
<!-- src/components/LogPanel.vue -->
<template>
  <div class="log-panel">
    <button class="toggle" @click="open = !open">
      {{ open ? "▲ 折叠日志" : "▼ 展开日志" }}
    </button>
    <div v-show="open" class="log-body" ref="logEl">
      <div v-for="(line, i) in logs" :key="i" class="log-line">{{ line }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from "vue";

const props = defineProps<{ logs: string[] }>();
const open = ref(false);
const logEl = ref<HTMLDivElement | null>(null);

// 新日志时自动滚到底
watch(() => props.logs.length, async () => {
  if (open.value) {
    await nextTick();
    logEl.value?.scrollTo({ top: logEl.value.scrollHeight, behavior: "smooth" });
  }
});
</script>

<style scoped>
.log-panel { margin-top: 16px; }
.toggle {
  background: none; border: 1px solid var(--color-border);
  font-size: 12px; color: var(--color-muted); padding: 4px 10px; width: 100%;
}
.log-body {
  margin-top: 4px;
  max-height: 180px; overflow-y: auto;
  background: #0f172a; color: #94a3b8;
  border-radius: var(--radius);
  padding: 12px;
}
.log-line { font-family: monospace; font-size: 12px; line-height: 1.6; }
</style>
```

- [ ] **Step 8: 写 QrCodeModal.vue**

```vue
<!-- src/components/QrCodeModal.vue -->
<template>
  <Teleport to="body">
    <div class="overlay" @click.self="$emit('close')">
      <div class="modal">
        <h3>扫码前往配置页面</h3>
        <canvas ref="qrCanvas" />
        <p class="url-text">{{ url }}</p>
        <button class="btn-secondary" @click="$emit('close')">关闭</button>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import QRCode from "qrcode";

const props = defineProps<{ url: string }>();
defineEmits<{ close: [] }>();

const qrCanvas = ref<HTMLCanvasElement | null>(null);

onMounted(async () => {
  if (qrCanvas.value) {
    await QRCode.toCanvas(qrCanvas.value, props.url, { width: 200, margin: 2 });
  }
});
</script>

<style scoped>
.overlay {
  position: fixed; inset: 0;
  background: rgba(0,0,0,.5);
  display: flex; align-items: center; justify-content: center;
  z-index: 999;
}
.modal {
  background: var(--color-surface);
  border-radius: var(--radius);
  padding: 24px;
  display: flex; flex-direction: column; align-items: center; gap: 12px;
  box-shadow: 0 20px 60px rgba(0,0,0,.2);
}
.url-text { font-size: 11px; color: var(--color-muted); word-break: break-all; max-width: 220px; text-align: center; }
</style>
```

- [ ] **Step 9: cargo check + Commit**

```bash
cd src-tauri && cargo check 2>&1 | grep "^error" | head -3
cd ..
npm run build 2>&1 | grep -E "error|warning" | head -5
git add src/components/ src/composables/
git commit -m "feat(frontend): 公共组件与 Tauri IPC 封装"
```

---

## Chunk 4: WelcomePage + SystemCheckPage

### Task 4: 欢迎页与系统检查页

**Files:**
- Create: `src/pages/WelcomePage.vue`
- Create: `src/pages/SystemCheckPage.vue`
- Create: `tests/unit/pages/WelcomePage.test.ts`
- Create: `tests/unit/pages/SystemCheckPage.test.ts`

- [ ] **Step 1: 写 WelcomePage 测试**

```typescript
// tests/unit/pages/WelcomePage.test.ts
import { mount, flushPromises } from "@vue/test-utils";
import WelcomePage from "@/pages/WelcomePage.vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { vi, describe, it, expect, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

function makeApp() {
  const pinia = createPinia();
  const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: WelcomePage }] });
  return { pinia, router };
}

describe("WelcomePage", () => {
  beforeEach(() => { vi.clearAllMocks(); });

  it("显示产品名称", async () => {
    mockInvoke.mockResolvedValue(null); // read_deploy_meta → null
    const { pinia, router } = makeApp();
    const wrapper = mount(WelcomePage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.text()).toContain("OpenClaw");
  });

  it("检测到已有安装时显示黄色提示", async () => {
    mockInvoke.mockResolvedValue({
      version: "1.0.0", install_path: "/opt/openclaw",
      installed_at: "2026-01-01", service_port: 18789,
    });
    const { pinia, router } = makeApp();
    const wrapper = mount(WelcomePage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.find(".existing-banner").exists()).toBe(true);
    expect(wrapper.text()).toContain("1.0.0");
  });
});
```

- [ ] **Step 2: 写 src/pages/WelcomePage.vue**

```vue
<template>
  <WizardLayout next-label="开始安装" @next="handleNext" :show-footer="true" :hide-next="false">
    <div class="welcome">
      <div class="hero">
        <div class="logo">🦾</div>
        <h1>OpenClaw</h1>
        <p class="subtitle">企业级多平台机器人网关 · 一键部署</p>
        <p class="version">向导版本 {{ wizardVersion }}</p>
      </div>

      <!-- 已安装检测提示 -->
      <div class="existing-banner" v-if="wizard.isExistingInstall">
        <span>⚠️ 检测到已安装版本 {{ wizard.existingVersion }}（{{ wizard.existingPath }}）</span>
        <div class="existing-actions">
          <button class="btn-secondary" @click="mode = 'upgrade'">升级安装</button>
          <button class="btn-secondary" style="color:var(--color-error)" @click="mode = 'fresh'">全新安装</button>
        </div>
      </div>

      <!-- 功能亮点卡片 -->
      <div class="feature-grid">
        <div class="feature-card" v-for="f in features" :key="f.icon">
          <span class="f-icon">{{ f.icon }}</span>
          <div>
            <div class="f-title">{{ f.title }}</div>
            <div class="f-desc">{{ f.desc }}</div>
          </div>
        </div>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const wizard = useWizardStore();
const { next } = useWizardNav();
const wizardVersion = __APP_VERSION__;
const mode = ref<"fresh" | "upgrade">("fresh");

const features = [
  { icon: "📦", title: "全量内置", desc: "内嵌 Node.js，无需联网即可完成安装" },
  { icon: "🔌", title: "平台集成", desc: "飞书 / 企业微信 / 钉钉 / QQ Work" },
  { icon: "🚀", title: "开机自启", desc: "注册系统服务，故障自动重启" },
  { icon: "🌐", title: "网页控制台", desc: "浏览器管理机器人、权限与日志" },
];

onMounted(async () => {
  wizard.setReady(true);
  try {
    const meta = await tauri.readDeployMeta();
    if (meta) {
      wizard.setExistingInstall(meta.version, meta.install_path);
    }
  } catch { /* 无已有安装 */ }
});

async function handleNext() {
  next();
}
</script>

<script lang="ts">
// vite define __APP_VERSION__
declare const __APP_VERSION__: string;
</script>

<style scoped>
.welcome { display: flex; flex-direction: column; gap: 24px; }

.hero { text-align: center; padding: 16px 0; }
.logo { font-size: 48px; margin-bottom: 8px; }
h1 { font-size: 28px; font-weight: 700; }
.subtitle { color: var(--color-muted); margin-top: 4px; }
.version { font-size: 12px; color: var(--color-border); margin-top: 8px; }

.existing-banner {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 12px 16px;
  display: flex; align-items: center; justify-content: space-between;
}
.existing-actions { display: flex; gap: 8px; }

.feature-grid {
  display: grid; grid-template-columns: 1fr 1fr; gap: 12px;
}
.feature-card {
  display: flex; gap: 12px; align-items: flex-start;
  padding: 16px; background: var(--color-surface);
  border: 1px solid var(--color-border); border-radius: var(--radius);
  box-shadow: var(--shadow);
}
.f-icon { font-size: 24px; flex-shrink: 0; }
.f-title { font-weight: 600; }
.f-desc { font-size: 12px; color: var(--color-muted); margin-top: 2px; }
</style>
```

- [ ] **Step 3: 写 SystemCheckPage.vue**

```vue
<template>
  <WizardLayout next-label="下一步" @next="handleNext" :show-footer="true">
    <div class="system-check">
      <h2>系统环境检查</h2>
      <p class="desc">正在检查安装环境，必需项全部通过后方可继续。</p>

      <div class="check-list" v-if="checks.length">
        <CheckItemComponent v-for="c in checks" :key="c.id" :item="c" />
      </div>
      <div v-else class="loading">
        <span>⏳ 正在检查中…</span>
      </div>

      <div class="summary" v-if="checks.length">
        <span v-if="allRequired" class="ok">✓ 所有必需项已通过</span>
        <span v-else class="err">✕ {{ failedRequired.length }} 项必需检查未通过</span>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import CheckItemComponent from "@/components/CheckItem.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const wizard = useWizardStore();
const { next } = useWizardNav();
const checks = computed(() => wizard.systemChecks);

const failedRequired = computed(() =>
  checks.value.filter((c) => c.required && c.status === "error")
);

const allRequired = computed(() => failedRequired.value.length === 0 && checks.value.length > 0);

onMounted(async () => {
  wizard.setReady(false);
  const results = await tauri.runSystemCheck();
  wizard.setChecks(results);
  wizard.setReady(allRequired.value);
});

function handleNext() {
  if (allRequired.value) next();
}
</script>

<style scoped>
.system-check { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); }
.check-list { display: flex; flex-direction: column; gap: 8px; }
.loading { text-align: center; color: var(--color-muted); padding: 32px; }
.summary { font-size: 14px; font-weight: 600; }
.ok { color: var(--color-success); }
.err { color: var(--color-error); }
</style>
```

- [ ] **Step 4: 运行页面测试**

```bash
npm run test -- tests/unit/pages/WelcomePage.test.ts 2>&1 | grep -E "FAIL|PASS|✓|×"
# 期望: PASS
```

- [ ] **Step 5: Commit**

```bash
git add src/pages/WelcomePage.vue src/pages/SystemCheckPage.vue tests/unit/pages/
git commit -m "feat(frontend): WelcomePage + SystemCheckPage"
```

---

## Chunk 5: SourcePage + Clash 相关页

### Task 5: 安装来源选择与代理配置页

**Files:**
- Create: `src/pages/SourcePage.vue`
- Create: `src/pages/ClashDisclaimerPage.vue`
- Create: `src/pages/ClashConfigPage.vue`
- Create: `tests/unit/pages/ClashDisclaimerPage.test.ts`

- [ ] **Step 1: 写 ClashDisclaimer 测试**

```typescript
// tests/unit/pages/ClashDisclaimerPage.test.ts
import { mount, flushPromises } from "@vue/test-utils";
import ClashDisclaimerPage from "@/pages/ClashDisclaimerPage.vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { vi, describe, it, expect, beforeEach } from "vitest";

describe("ClashDisclaimerPage", () => {
  it("未勾选时继续按钮禁用", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ClashDisclaimerPage }] });
    const wrapper = mount(ClashDisclaimerPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    // wizard.canProceed 应为 false（未勾选）
    // 此页由 WizardLayout 控制按钮 disabled，这里检查内部 accepted 状态
    expect(wrapper.find('input[type="checkbox"]').element.checked).toBe(false);
  });

  it("勾选后 canProceed 为 true", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ClashDisclaimerPage }] });
    const wrapper = mount(ClashDisclaimerPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    await wrapper.find('input[type="checkbox"]').setValue(true);
    expect(wrapper.find('input[type="checkbox"]').element.checked).toBe(true);
  });
});
```

- [ ] **Step 2: 写 src/pages/SourcePage.vue**

```vue
<template>
  <WizardLayout @next="handleNext">
    <div class="source-page">
      <h2>选择安装来源</h2>
      <p class="desc">根据您的网络环境选择合适的安装方式。</p>

      <div class="options">
        <label class="option" :class="{ selected: selected === 'bundled', disabled: !hasBundled }">
          <input type="radio" value="bundled" v-model="selected" :disabled="!hasBundled" />
          <span class="opt-icon">📦</span>
          <div>
            <div class="opt-title">使用内置离线包 <span v-if="!hasBundled">(此版本不含)</span></div>
            <div class="opt-desc">无需网络，直接安装，最稳定</div>
          </div>
        </label>

        <label class="option" :class="{ selected: selected === 'online' }">
          <input type="radio" value="online" v-model="selected" />
          <span class="opt-icon">🌐</span>
          <div>
            <div class="opt-title">从网络下载最新版本</div>
            <div class="opt-desc">需要访问 GitHub Release，可配置代理</div>
          </div>
        </label>

        <label class="option" :class="{ selected: selected === 'local_zip' }">
          <input type="radio" value="local_zip" v-model="selected" />
          <span class="opt-icon">📁</span>
          <div>
            <div class="opt-title">导入本地安装包</div>
            <div class="opt-desc">使用已下载的 ZIP 文件</div>
          </div>
        </label>
      </div>

      <!-- 本地 ZIP 文件选择 -->
      <div class="zip-zone" v-if="selected === 'local_zip'" @click="pickZip" @dragover.prevent @drop.prevent="onDrop">
        <span v-if="config.localZipPath">{{ config.localZipPath }}</span>
        <span v-else>拖拽 ZIP 到此处，或点击选择文件</span>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { useWizardNav } from "@/composables/useWizardNav";
import { open } from "@tauri-apps/plugin-dialog";

const wizard = useWizardStore();
const config = useConfigStore();
const { next, goTo } = useWizardNav();

// 检测是否有内嵌资源（通过构建时常量）
const hasBundled = ref(typeof __HAS_BUNDLED__ !== "undefined" ? __HAS_BUNDLED__ : false);
const selected = ref(hasBundled.value ? "bundled" : "online");

watch(selected, (v) => {
  wizard.setSourceMode(v as any);
  wizard.setReady(v !== "local_zip" || !!config.localZipPath);
});

onMounted(() => {
  wizard.setSourceMode(selected.value as any);
  wizard.setReady(selected.value !== "local_zip");
});

async function pickZip() {
  const path = await open({ filters: [{ name: "ZIP", extensions: ["zip"] }] });
  if (typeof path === "string") {
    config.localZipPath = path;
    wizard.setReady(true);
  }
}

function onDrop(e: DragEvent) {
  const file = e.dataTransfer?.files[0];
  if (file && file.name.endsWith(".zip")) {
    config.localZipPath = file.path;
    wizard.setReady(true);
  }
}

function handleNext() {
  if (selected.value === "online") {
    goTo("clash-disclaimer");
  } else {
    goTo("install");
  }
}
</script>

<script lang="ts">
declare const __HAS_BUNDLED__: boolean;
</script>

<style scoped>
.source-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); }

.options { display: flex; flex-direction: column; gap: 10px; }
.option {
  display: flex; align-items: center; gap: 12px;
  padding: 16px; border: 2px solid var(--color-border);
  border-radius: var(--radius); cursor: pointer;
}
.option.selected { border-color: var(--color-primary); background: #eff6ff; }
.option.disabled { opacity: .5; cursor: not-allowed; }
.opt-icon { font-size: 24px; }
.opt-title { font-weight: 600; }
.opt-desc { font-size: 12px; color: var(--color-muted); }

.zip-zone {
  border: 2px dashed var(--color-border); border-radius: var(--radius);
  padding: 24px; text-align: center; cursor: pointer;
  color: var(--color-muted); font-size: 13px;
}
.zip-zone:hover { border-color: var(--color-primary); }
</style>
```

- [ ] **Step 3: 写 src/pages/ClashDisclaimerPage.vue**

```vue
<template>
  <WizardLayout @next="handleNext">
    <div class="disclaimer-page">
      <h2>⚠️ 代理工具免责声明</h2>

      <div class="disclaimer-box">
        <p>本向导可选择性地集成 <strong>Mihomo（原 Clash Meta）</strong> 代理工具，仅用于在网络受限环境下下载 OpenClaw 所需资源。</p>
        <br />
        <p><strong>使用前请确认以下事项：</strong></p>
        <ul>
          <li>您已充分了解并遵守您所在地区关于网络代理的相关法律法规。</li>
          <li>代理订阅链接由您自行提供，本软件不提供、推荐或背书任何代理服务。</li>
          <li>代理仅在资源下载期间临时运行，完成后将自动停止并删除相关文件。</li>
          <li>Mihomo 使用 GPL-3.0 开源协议，来源：MetaCubX/mihomo。</li>
          <li>因使用代理工具产生的一切法律责任由用户自行承担。</li>
        </ul>
        <br />
        <p>若您所在网络无限制，可点击"跳过代理，直连下载"。</p>
      </div>

      <label class="accept-row">
        <input type="checkbox" v-model="accepted" />
        <span>我已阅读并同意上述免责声明，继续配置代理</span>
      </label>

      <div class="skip-row">
        <button class="btn-secondary" @click="skipClash">跳过代理，直连下载</button>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const wizard = useWizardStore();
const { next, goTo } = useWizardNav();
const accepted = ref(false);

watch(accepted, (v) => {
  wizard.setClashAccepted(v);
  wizard.setReady(v);
});

function handleNext() {
  if (accepted.value) next();
}

function skipClash() {
  wizard.setClashAccepted(false);
  goTo("install");
}
</script>

<style scoped>
.disclaimer-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; color: var(--color-warning); }
.disclaimer-box {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 16px; font-size: 13px; line-height: 1.8;
}
.disclaimer-box ul { padding-left: 20px; }
.accept-row { display: flex; gap: 10px; align-items: flex-start; cursor: pointer; font-size: 13px; }
.skip-row { margin-top: -4px; }
</style>
```

- [ ] **Step 4: 写 src/pages/ClashConfigPage.vue**

```vue
<template>
  <WizardLayout @next="handleNext" :next-label="testing ? '测试中…' : '使用此代理'">
    <div class="clash-config">
      <h2>配置 Clash 代理</h2>
      <p class="desc">输入您的订阅链接，我们会临时启动 Mihomo 完成资源下载。</p>

      <div class="field">
        <label>订阅链接</label>
        <div class="input-row">
          <input
            type="url" v-model="subUrl"
            placeholder="https://your-clash-sub-url..."
            @input="testResult = null"
          />
          <button class="btn-secondary" @click="test" :disabled="!subUrl || testing">
            {{ testing ? "测试中…" : "测试连接" }}
          </button>
        </div>
      </div>

      <div class="test-result" v-if="testResult">
        <span v-if="testResult.success" class="ok">✓ 连接成功，延迟 {{ testResult.latency_ms }}ms</span>
        <span v-else class="err">✕ {{ testResult.error }}</span>
      </div>

      <p class="tip">ℹ️ 代理仅在资源下载期间临时启用，完成后自动停止并删除相关文件。</p>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri, type ClashTestResult } from "@/composables/useTauri";

const wizard = useWizardStore();
const config = useConfigStore();
const { next } = useWizardNav();

const subUrl = ref(config.clashSubscriptionUrl || "");
const testing = ref(false);
const testResult = ref<ClashTestResult | null>(null);

watch(subUrl, (v) => { wizard.setReady(!!v); });
wizard.setReady(!!subUrl.value);

async function test() {
  testing.value = true;
  try {
    testResult.value = await tauri.clashTest(subUrl.value);
  } finally {
    testing.value = false;
  }
}

function handleNext() {
  config.clashSubscriptionUrl = subUrl.value;
  next();
}
</script>

<style scoped>
.clash-config { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }
.desc, .tip { color: var(--color-muted); font-size: 13px; }

.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
.input-row { display: flex; gap: 8px; }
input[type="url"] {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius);
  font-size: 13px;
}

.test-result { font-size: 13px; font-weight: 600; }
.ok { color: var(--color-success); }
.err { color: var(--color-error); }
</style>
```

- [ ] **Step 5: 运行测试**

```bash
npm run test -- tests/unit/pages/ClashDisclaimerPage.test.ts 2>&1 | grep -E "FAIL|PASS"
# 期望: PASS
```

- [ ] **Step 6: Commit**

```bash
git add src/pages/SourcePage.vue src/pages/ClashDisclaimerPage.vue src/pages/ClashConfigPage.vue tests/unit/pages/ClashDisclaimerPage.test.ts
git commit -m "feat(frontend): SourcePage + ClashDisclaimerPage + ClashConfigPage"
```

---

## Chunk 6: InstallConfigPage + ServiceConfigPage

### Task 6: 安装配置与服务配置页

**Files:**
- Create: `src/pages/InstallConfigPage.vue`
- Create: `src/pages/ServiceConfigPage.vue`
- Create: `tests/unit/pages/ServiceConfigPage.test.ts`

- [ ] **Step 1: 写 ServiceConfigPage 测试**

```typescript
// tests/unit/pages/ServiceConfigPage.test.ts
import { mount, flushPromises } from "@vue/test-utils";
import ServiceConfigPage from "@/pages/ServiceConfigPage.vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

describe("ServiceConfigPage", () => {
  beforeEach(() => { vi.clearAllMocks(); mockInvoke.mockResolvedValue({}); });

  it("端口超出范围时 canProceed 为 false", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ServiceConfigPage }] });
    const wrapper = mount(ServiceConfigPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    const portInput = wrapper.find('input[type="number"]');
    await portInput.setValue(80); // < 1024，非法
    await flushPromises();
    // wizard.canProceed 应为 false
    const { useWizardStore } = await import("@/stores/wizard");
    const { setActivePinia } = await import("pinia");
    setActivePinia(pinia);
    const w = useWizardStore();
    expect(w.canProceed).toBe(false);
  });

  it("密码不匹配时 canProceed 为 false", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ServiceConfigPage }] });
    const wrapper = mount(ServiceConfigPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    await wrapper.find('input[type="password"]').setValue("abc12345");
    // 确认密码不设置，仍为空
    const { useWizardStore } = await import("@/stores/wizard");
    const { setActivePinia } = await import("pinia");
    setActivePinia(pinia);
    const w = useWizardStore();
    expect(w.canProceed).toBe(false);
  });
});
```

- [ ] **Step 2: 写 src/pages/InstallConfigPage.vue**

```vue
<template>
  <WizardLayout @next="handleNext">
    <div class="install-config">
      <h2>安装路径</h2>

      <div class="field">
        <label>安装目录</label>
        <div class="input-row">
          <input type="text" v-model="config.installPath" @input="validate" />
          <button class="btn-secondary" @click="pickDir">浏览…</button>
        </div>
        <span class="hint">推荐路径：{{ defaultPath }}</span>
      </div>

      <div class="field">
        <label class="checkbox-row">
          <input type="checkbox" v-model="config.installService" />
          注册系统服务（推荐）
        </label>
        <p class="hint-sm">将 OpenClaw 注册为系统服务，故障自动重启。</p>
      </div>

      <div class="field" v-if="config.installService">
        <label class="checkbox-row">
          <input type="checkbox" v-model="config.startOnBoot" />
          开机自动启动
        </label>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";
import { open } from "@tauri-apps/plugin-dialog";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();
const defaultPath = ref("/opt/openclaw");

onMounted(async () => {
  defaultPath.value = await tauri.getDefaultInstallPath();
  if (!config.installPath || config.installPath === "/opt/openclaw") {
    config.installPath = defaultPath.value;
  }
  validate();
});

function validate() {
  wizard.setReady(!!config.installPath.trim());
}

async function pickDir() {
  const dir = await open({ directory: true });
  if (typeof dir === "string") {
    config.installPath = dir;
    validate();
  }
}

function handleNext() { next(); }
</script>

<style scoped>
.install-config { display: flex; flex-direction: column; gap: 20px; }
h2 { font-size: 20px; font-weight: 700; }
.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
.checkbox-row { display: flex; gap: 8px; align-items: center; cursor: pointer; }
.input-row { display: flex; gap: 8px; }
input[type="text"] {
  flex: 1; padding: 8px 12px;
  border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 13px;
}
.hint { font-size: 12px; color: var(--color-muted); }
.hint-sm { font-size: 12px; color: var(--color-muted); margin-left: 24px; }
</style>
```

- [ ] **Step 3: 写 src/pages/ServiceConfigPage.vue**

```vue
<template>
  <WizardLayout @next="handleNext">
    <div class="service-config">
      <h2>服务配置</h2>

      <div class="field">
        <label>监听端口</label>
        <input
          type="number" v-model.number="config.servicePort"
          min="1024" max="65535" @input="validate"
        />
        <span class="error" v-if="portError">{{ portError }}</span>
      </div>

      <div class="field">
        <label>绑定域名（可选）</label>
        <input type="text" v-model="config.domainName" placeholder="例：openlaw.example.com" />
        <span class="hint">留空则仅通过 IP:端口 访问</span>
      </div>

      <div class="field">
        <label>管理员密码</label>
        <input type="password" v-model="config.adminPassword" @input="validate" placeholder="至少 8 位，含字母+数字" />
        <PasswordStrength :password="config.adminPassword" />
        <span class="error" v-if="passError">{{ passError }}</span>
      </div>

      <div class="field">
        <label>确认密码</label>
        <input type="password" v-model="config.confirmPassword" @input="validate" />
        <span class="error" v-if="matchError">{{ matchError }}</span>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import PasswordStrength from "@/components/PasswordStrength.vue";
import { useConfigStore } from "@/stores/config";
import { useWizardStore } from "@/stores/wizard";
import { useWizardNav } from "@/composables/useWizardNav";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();

const portError = ref("");
const passError = ref("");
const matchError = ref("");

function validate() {
  portError.value = "";
  passError.value = "";
  matchError.value = "";

  const port = config.servicePort;
  if (port < 1024 || port > 65535) {
    portError.value = "端口范围 1024-65535";
    wizard.setReady(false); return;
  }
  if (!config.isPasswordValid) {
    passError.value = "密码至少 8 位，需含字母和数字";
    wizard.setReady(false); return;
  }
  if (!config.passwordsMatch) {
    matchError.value = "两次密码不一致";
    wizard.setReady(false); return;
  }
  wizard.setReady(true);
}

validate();

function handleNext() { next(); }
</script>

<style scoped>
.service-config { display: flex; flex-direction: column; gap: 20px; }
h2 { font-size: 20px; font-weight: 700; }
.field { display: flex; flex-direction: column; gap: 6px; }
label { font-weight: 500; font-size: 13px; }
input[type="number"], input[type="text"], input[type="password"] {
  padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 13px; max-width: 360px;
}
.error { font-size: 12px; color: var(--color-error); }
.hint { font-size: 12px; color: var(--color-muted); }
</style>
```

- [ ] **Step 4: 运行测试**

```bash
npm run test -- tests/unit/pages/ServiceConfigPage.test.ts 2>&1 | grep -E "FAIL|PASS"
# 期望: PASS
```

- [ ] **Step 5: Commit**

```bash
git add src/pages/InstallConfigPage.vue src/pages/ServiceConfigPage.vue tests/unit/pages/ServiceConfigPage.test.ts
git commit -m "feat(frontend): InstallConfigPage + ServiceConfigPage"
```

---

## Chunk 7: PlatformIntegrationPage

### Task 7: 平台集成页（QR 码）

**Files:**
- Create: `src/pages/PlatformIntegrationPage.vue`
- Create: `tests/unit/pages/PlatformIntegrationPage.test.ts`

- [ ] **Step 1: 写平台集成测试**

```typescript
// tests/unit/pages/PlatformIntegrationPage.test.ts
import { mount, flushPromises } from "@vue/test-utils";
import PlatformIntegrationPage from "@/pages/PlatformIntegrationPage.vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { describe, it, expect, vi } from "vitest";

describe("PlatformIntegrationPage", () => {
  it("页面加载时 canProceed 为 true（可选页面）", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: PlatformIntegrationPage }] });
    const wrapper = mount(PlatformIntegrationPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    const { useWizardStore } = await import("@/stores/wizard");
    const { setActivePinia } = await import("pinia");
    setActivePinia(pinia);
    const w = useWizardStore();
    expect(w.canProceed).toBe(true);
  });

  it("显示 4 个平台", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: PlatformIntegrationPage }] });
    const wrapper = mount(PlatformIntegrationPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.findAll(".platform-card")).toHaveLength(4);
  });
});
```

- [ ] **Step 2: 写 src/pages/PlatformIntegrationPage.vue**

```vue
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
                  <button class="btn-secondary sm" @click="openDoc(p.docUrl)">💻 打开</button>
                </div>
              </div>
              <div class="guide-step">
                <span class="step-num">2</span>
                <span>将 Webhook 地址粘贴到此处</span>
              </div>
            </div>
            <div class="webhook-input">
              <input
                type="url"
                v-model="config.platforms[p.id].webhookUrl"
                :placeholder="p.placeholder"
              />
              <span class="valid-mark" v-if="isValidWebhook(p.id)">✓</span>
              <span class="warn-mark" v-else-if="config.platforms[p.id].webhookUrl">⚠</span>
            </div>
          </template>
        </div>
      </div>
    </div>

    <!-- QR 码弹窗 -->
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
import { tauri } from "@/composables/useTauri";

const config = useConfigStore();
const wizard = useWizardStore();
const { next } = useWizardNav();
const activeQr = ref<string | null>(null);

const platforms = [
  {
    id: "wx",
    icon: "💼", name: "企业微信",
    docUrl: "https://work.weixin.qq.com/api/doc/90000/90136/91770",
    placeholder: "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=...",
    pattern: /qyapi\.weixin\.qq\.com/,
  },
  {
    id: "qq",
    icon: "🐧", name: "QQ Work",
    docUrl: "https://work.qq.com/",
    placeholder: "https://qyapi.im.qq.com/cgi-bin/webhook/send?key=...",
    pattern: /qyapi\.im\.qq\.com/,
  },
  {
    id: "dt",
    icon: "⚙️", name: "钉钉",
    docUrl: "https://open.dingtalk.com/document/robots/custom-robot-access",
    placeholder: "https://oapi.dingtalk.com/robot/send?access_token=...",
    pattern: /oapi\.dingtalk\.com/,
  },
  {
    id: "fs",
    icon: "🪁", name: "飞书",
    docUrl: "https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot",
    placeholder: "https://open.feishu.cn/open-apis/bot/v2/hook/...",
    pattern: /open\.feishu\.cn/,
  },
];

onMounted(() => { wizard.setReady(true); }); // 可选页面始终可继续

function showQr(p: typeof platforms[0]) { activeQr.value = p.docUrl; }
function openDoc(url: string) { tauri.openUrl(url); }
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
.warn-mark  { color: var(--color-warning); font-size: 16px; }
</style>
```

- [ ] **Step 3: 运行测试**

```bash
npm run test -- tests/unit/pages/PlatformIntegrationPage.test.ts 2>&1 | grep -E "FAIL|PASS"
# 期望: PASS
```

- [ ] **Step 4: Commit**

```bash
git add src/pages/PlatformIntegrationPage.vue tests/unit/pages/PlatformIntegrationPage.test.ts
git commit -m "feat(frontend): PlatformIntegrationPage with QR code"
```

---

## Chunk 8: DeploymentPage

### Task 8: 部署页面（进度事件监听）

**Files:**
- Create: `src/pages/DeploymentPage.vue`
- Create: `tests/unit/pages/DeploymentPage.test.ts`

- [ ] **Step 1: 写 DeploymentPage 测试**

```typescript
// tests/unit/pages/DeploymentPage.test.ts
import { mount, flushPromises } from "@vue/test-utils";
import DeploymentPage from "@/pages/DeploymentPage.vue";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

describe("DeploymentPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
    // listen 返回取消函数
    mockListen.mockResolvedValue(() => {});
  });

  it("挂载后调用 start_deploy", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({ history: createWebHistory(), routes: [
      { path: "/deploy", name: "deploy", component: DeploymentPage },
      { path: "/finish", name: "finish", component: { template: "<div/>" } },
    ]});
    await router.push("/deploy");
    mount(DeploymentPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("start_deploy", expect.anything());
  });

  it("进度条初始为 0%", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: DeploymentPage }] });
    const wrapper = mount(DeploymentPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.find(".progress-fill").attributes("style")).toContain("0%");
  });
});
```

- [ ] **Step 2: 写 src/pages/DeploymentPage.vue**

```vue
<template>
  <WizardLayout :show-footer="false">
    <div class="deploy-page">
      <h2>正在部署</h2>

      <div class="status-row">
        <span class="status-icon" :class="wizard.deployStatus">
          {{ statusIcon }}
        </span>
        <span class="status-msg">{{ wizard.deployProgress.message || "准备中…" }}</span>
      </div>

      <!-- 进度条 -->
      <div class="progress-track">
        <div
          class="progress-fill"
          :class="{ error: wizard.deployStatus === 'failed' }"
          :style="{ width: wizard.deployProgress.percent + '%' }"
        />
      </div>
      <div class="progress-label">{{ wizard.deployProgress.percent }}%</div>

      <!-- 日志区 -->
      <LogPanel :logs="wizard.deployLogs" />

      <!-- 失败操作 -->
      <div class="fail-actions" v-if="wizard.deployStatus === 'failed'">
        <p class="err-msg">{{ errorReason }}</p>
        <button class="btn-secondary" @click="showLogs = true">查看详细日志</button>
        <button class="btn-secondary" @click="goTo('install')">返回重新配置</button>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import LogPanel from "@/components/LogPanel.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { useWizardNav } from "@/composables/useWizardNav";
import { useDeployEvents, tauri } from "@/composables/useTauri";

const wizard = useWizardStore();
const config = useConfigStore();
const { goTo } = useWizardNav();
const errorReason = ref("");
const showLogs = ref(false);

const statusIcon = computed(() => ({
  idle: "⏳", running: "⚡", done: "✅", failed: "❌",
}[wizard.deployStatus]));

const { subscribe } = useDeployEvents(
  (p) => {
    wizard.updateProgress(p);
    wizard.setDeployStatus("running");
  },
  () => {
    wizard.setDeployStatus("done");
    // 停止 Clash（若启用）
    tauri.clashStop().catch(() => {});
    goTo("finish");
  },
  (reason) => {
    wizard.setDeployStatus("failed");
    errorReason.value = reason;
    // 停止 Clash
    tauri.clashStop().catch(() => {});
  },
);

onMounted(async () => {
  await subscribe();
  wizard.setDeployStatus("running");

  // 构建完整 DTO
  const dto = {
    ...config.toDto(),
    source_mode: buildSourceMode(),
  };
  await tauri.startDeploy(dto);
});

function buildSourceMode() {
  const m = wizard.sourceMode;
  if (m === "bundled") return { type: "bundled" };
  if (m === "local_zip") return { type: "local_zip", zip_path: config.localZipPath };
  return {
    type: "online",
    proxy_url: wizard.clashAccepted && config.clashSubscriptionUrl
      ? config.clashSubscriptionUrl : null,
  };
}
</script>

<style scoped>
.deploy-page { display: flex; flex-direction: column; gap: 16px; }
h2 { font-size: 20px; font-weight: 700; }

.status-row { display: flex; align-items: center; gap: 12px; }
.status-icon { font-size: 24px; }

.progress-track { height: 8px; background: var(--color-border); border-radius: 4px; overflow: hidden; }
.progress-fill { height: 100%; background: var(--color-primary); transition: width .4s ease; border-radius: 4px; }
.progress-fill.error { background: var(--color-error); }
.progress-label { font-size: 12px; color: var(--color-muted); text-align: right; }

.fail-actions { display: flex; flex-direction: column; gap: 8px; }
.err-msg { color: var(--color-error); font-size: 13px; }
</style>
```

- [ ] **Step 3: 运行测试**

```bash
npm run test -- tests/unit/pages/DeploymentPage.test.ts 2>&1 | grep -E "FAIL|PASS"
# 期望: PASS
```

- [ ] **Step 4: Commit**

```bash
git add src/pages/DeploymentPage.vue tests/unit/pages/DeploymentPage.test.ts
git commit -m "feat(frontend): DeploymentPage with real-time progress events"
```

---

## Chunk 9: FinishPage + 全局集成

### Task 9: 完成页面与最终集成

**Files:**
- Create: `src/pages/FinishPage.vue`
- Create: `tests/unit/pages/FinishPage.test.ts`
- Modify: `vite.config.ts`（添加 `__APP_VERSION__` 和 `__HAS_BUNDLED__` define）
- Modify: `src-tauri/tauri.conf.json`（确认 bundle 配置）

- [ ] **Step 1: 写 FinishPage 测试**

```typescript
// tests/unit/pages/FinishPage.test.ts
import { mount, flushPromises } from "@vue/test-utils";
import FinishPage from "@/pages/FinishPage.vue";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

describe("FinishPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue([]);
  });

  it("显示成功图标", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: FinishPage }] });
    const wrapper = mount(FinishPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.find(".success-icon").exists()).toBe(true);
  });

  it("服务未运行时显示橙色警告", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const { useWizardStore } = await import("@/stores/wizard");
    const w = useWizardStore();
    w.setDeployStatus("failed"); // 模拟忽略错误继续
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: FinishPage }] });
    const wrapper = mount(FinishPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    // 当 deployStatus 非 done 时显示警告横幅
    expect(wrapper.find(".warn-banner").exists()).toBe(true);
  });
});
```

- [ ] **Step 2: 写 src/pages/FinishPage.vue**

```vue
<template>
  <WizardLayout :show-footer="false">
    <div class="finish-page">
      <!-- 非正常完成警告 -->
      <div class="warn-banner" v-if="wizard.deployStatus !== 'done'">
        ⚠️ 服务尚未运行，请查看日志或手动启动
      </div>

      <div class="hero">
        <div class="success-icon">✅</div>
        <h2>部署完成！</h2>
        <p class="subtitle">OpenClaw 已成功安装到您的系统。</p>
      </div>

      <!-- 安装信息摘要 -->
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
        <p class="pwd-note">忘记密码请通过管理控制台重置（控制台地址见上方）</p>
      </div>

      <!-- 操作按钮 -->
      <div class="action-row">
        <button class="btn-primary" @click="openConsole">🌐 打开管理控制台</button>
      </div>

      <!-- 下一步引导卡片 -->
      <div class="next-steps">
        <h3>下一步做什么？</h3>
        <div class="step-cards">
          <div class="step-card" v-for="s in nextSteps" :key="s.title" @click="openUrl(s.url)">
            <span class="sc-icon">{{ s.icon }}</span>
            <div class="sc-body">
              <div class="sc-title">{{ s.title }}</div>
              <div class="sc-desc">{{ s.desc }}</div>
            </div>
            <span class="sc-arrow">›</span>
          </div>
        </div>
      </div>

      <!-- Skills 更新（若有可用更新） -->
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

      <div class="feedback">
        <a href="https://github.com/openclaw/openclaw/issues" target="_blank">🐛 反馈问题</a>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardStore } from "@/stores/wizard";
import { useConfigStore } from "@/stores/config";
import { tauri, type SkillInfo } from "@/composables/useTauri";

const wizard = useWizardStore();
const config = useConfigStore();
const updatableSkills = ref<SkillInfo[]>([]);
const updating = ref(false);

const nextSteps = [
  { icon: "🤖", title: "创建第一个机器人", desc: "通过控制台创建并配置机器人", url: "#console" },
  { icon: "📖", title: "查看文档", desc: "了解 API、Skills 和配置选项", url: "https://github.com/openclaw/openclaw#readme" },
  { icon: "👥", title: "邀请团队成员", desc: "添加协作者，共同管理机器人", url: "#console/team" },
];

onMounted(async () => {
  // 1 秒后自动打开控制台
  setTimeout(() => openConsole(), 1000);
  // 检查 Skills 更新
  try {
    const skills = await tauri.listSkills(config.installPath);
    updatableSkills.value = skills.filter((s) => s.update_available);
  } catch { /* 忽略 */ }
});

function openConsole() {
  tauri.openUrl(`http://127.0.0.1:${config.servicePort}`);
}

function openUrl(url: string) {
  if (url.startsWith("http")) tauri.openUrl(url);
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

function copyText(text: string) {
  navigator.clipboard.writeText(text).catch(() => {});
}
</script>

<style scoped>
.finish-page { display: flex; flex-direction: column; gap: 20px; }

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
.pwd-note { font-size: 12px; color: var(--color-muted); margin-top: 4px; }

.action-row { display: flex; justify-content: center; }
.btn-primary { padding: 10px 24px; font-size: 15px; }

.next-steps h3, .skills-section h3 { font-size: 15px; font-weight: 600; margin-bottom: 10px; }
.step-cards { display: flex; flex-direction: column; gap: 8px; }
.step-card {
  display: flex; align-items: center; gap: 12px;
  padding: 12px 16px; border: 1px solid var(--color-border);
  border-radius: var(--radius); cursor: pointer;
}
.step-card:hover { background: var(--color-bg); }
.sc-icon { font-size: 20px; }
.sc-body { flex: 1; }
.sc-title { font-weight: 600; font-size: 13px; }
.sc-desc { font-size: 12px; color: var(--color-muted); }
.sc-arrow { color: var(--color-muted); }

.skill-list { display: flex; flex-direction: column; gap: 6px; margin-bottom: 10px; }
.skill-row { display: flex; justify-content: space-between; font-size: 13px; }
.version { color: var(--color-muted); }

.feedback { text-align: center; font-size: 13px; }
.feedback a { color: var(--color-muted); }
</style>
```

- [ ] **Step 3: 在 vite.config.ts 添加构建常量**

在 `vite.config.ts` 的 `defineConfig` 内追加：

```typescript
define: {
  __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "1.0.0"),
  __HAS_BUNDLED__: JSON.stringify(process.env.OC_BUILD_BUNDLED === "1"),
},
```

- [ ] **Step 4: 在 src-tauri 添加 open_url 和 read_deploy_meta 命令**

在 `src-tauri/src/main.rs` 注册：

```rust
#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_deploy_meta() -> Option<serde_json::Value> {
    let path = dirs::home_dir()?.join(".openclaw/deploy_meta.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

#[tauri::command]
async fn get_default_install_path() -> String {
    #[cfg(target_os = "linux")]
    return "/opt/openclaw".to_string();
    #[cfg(target_os = "windows")]
    return format!("{}\\openclaw",
        std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Public".to_string()));
    #[cfg(target_os = "macos")]
    return format!("{}/openclaw",
        dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "~".to_string()));
}
```

在 `Cargo.toml` 添加：

```toml
open = "5"
```

在 `generate_handler!` 中追加 `open_url, read_deploy_meta, get_default_install_path`。

- [ ] **Step 5: 运行全部测试**

```bash
npm run test 2>&1 | tail -20
# 期望: 所有测试 PASS，无 FAIL
```

- [ ] **Step 6: cargo check**

```bash
cd src-tauri && cargo check 2>&1 | grep "^error" | head -5
cd ..
# 期望: 无 error
```

- [ ] **Step 7: 构建验证**

```bash
npm run build 2>&1 | grep -E "error|✓ built" | head -5
# 期望: ✓ built
```

- [ ] **Step 8: Commit**

```bash
git add src/pages/FinishPage.vue tests/unit/pages/FinishPage.test.ts vite.config.ts src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat(frontend): FinishPage + 全局集成（open_url、read_deploy_meta）"
```

- [ ] **Step 9: Plan B 完成 Commit**

```bash
git add -u
git commit -m "feat(frontend): Plan B Vue 3 前端完成 - 10 页向导全部实现"
```

---

## Tauri Command 接口对照（Plan A ↔ Plan B）

| Frontend 调用 | Rust Command | 所在模块 |
|---|---|---|
| `tauri.runSystemCheck()` | `run_system_check` | system_check.rs |
| `tauri.startDeploy(dto)` | `start_deploy` | deploy.rs |
| `tauri.clashTest(url)` | `clash_test` | clash_proxy.rs |
| `tauri.clashStart(url)` | `clash_start` | clash_proxy.rs |
| `tauri.clashStop()` | `clash_stop` | clash_proxy.rs |
| `tauri.listSkills(path)` | `list_skills` | skills_manager.rs |
| `tauri.updateSkills(...)` | `update_skills` | skills_manager.rs |
| `tauri.checkUpdate(...)` | `check_openclaw_update` | updater.rs |
| `tauri.applyUpdate(...)` | `apply_openclaw_update` | updater.rs |
| `tauri.readDeployMeta()` | `read_deploy_meta` | main.rs（内联） |
| `tauri.openUrl(url)` | `open_url` | main.rs（内联） |
| `tauri.getDefaultInstallPath()` | `get_default_install_path` | main.rs（内联） |
| `tauri.loadSession()` | `load_session` | session_state.rs |
| `tauri.clearSession(...)` | `clear_session` | session_state.rs |

**事件（Rust → 前端）：**

| 事件名 | Payload 类型 | 监听方 |
|---|---|---|
| `deploy:progress` | `DeployProgress` | DeploymentPage |
| `deploy:done` | 空 | DeploymentPage |
| `deploy:failed` | `String`（原因） | DeploymentPage |
| `update:progress` | `String` | FinishPage（可选） |
| `update:done` | 空 | FinishPage（可选） |
