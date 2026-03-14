import { defineStore } from "pinia";
import { ref, computed } from "vue";

export interface PlatformConfig {
  enabled: boolean;
  webhookUrl: string;
}

export interface AiConfigDto {
  provider: string;
  base_url: string;
  api_key: string;
  model: string;
}

/** Rust PlatformEntry 的 JSON 形式：枚举名小写，字段 snake_case */
export interface PlatformEntry {
  platform: "wework" | "dingtalk" | "feishu";
  webhook_url: string;
}

/** QQ 开放平台机器人凭据（与其他平台不同，使用 AppID + AppSecret） */
export interface QqConfigDto {
  app_id: string;
  app_secret: string;
}

export interface DeployConfigDto {
  install_path: string;
  service_port: number;
  admin_password: string;
  domain_name: string | null;
  install_service: boolean;
  start_on_boot: boolean;
  source_mode: { type: string; proxy_url?: string; path?: string };
  platforms: PlatformEntry[];
  qq_config: QqConfigDto | null;
  ai_config: AiConfigDto | null;
}

const PLATFORM_ID_MAP: Record<string, "wework" | "dingtalk" | "feishu"> = {
  wx: "wework",
  dt: "dingtalk",
  fs: "feishu",
};

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
    dt: { enabled: false, webhookUrl: "" },
    fs: { enabled: false, webhookUrl: "" },
  });

  // QQ 机器人（AppID + AppSecret，与 Webhook 平台不同）
  const qqEnabled  = ref(false);
  const qqAppId    = ref("");
  const qqAppSecret = ref("");

  // AI 模型配置
  const aiProvider = ref("");
  const aiBaseUrl  = ref("");
  const aiApiKey   = ref("");
  const aiModel    = ref("");

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
    // 将 Record<id, PlatformConfig> 转换为 Rust 期望的 Vec<PlatformEntry>
    const platformEntries: PlatformEntry[] = Object.entries(platforms.value)
      .filter(([id, p]) => p.enabled && p.webhookUrl && PLATFORM_ID_MAP[id])
      .map(([id, p]) => ({
        platform: PLATFORM_ID_MAP[id],
        webhook_url: p.webhookUrl,
      }));

    return {
      install_path: installPath.value,
      service_port: servicePort.value,
      admin_password: adminPassword.value,
      domain_name: domainName.value,
      install_service: installService.value,
      start_on_boot: startOnBoot.value,
      source_mode: { type: "bundled" },
      platforms: platformEntries,
      qq_config: qqEnabled.value && qqAppId.value && qqAppSecret.value
        ? { app_id: qqAppId.value, app_secret: qqAppSecret.value }
        : null,
      ai_config: aiApiKey.value
        ? { provider: aiProvider.value, base_url: aiBaseUrl.value,
            api_key: aiApiKey.value, model: aiModel.value }
        : null,
    };
  }

  return {
    installPath, servicePort, adminPassword, confirmPassword,
    domainName, installService, startOnBoot, clashSubscriptionUrl,
    localZipPath, platforms,
    aiProvider, aiBaseUrl, aiApiKey, aiModel,
    qqEnabled, qqAppId, qqAppSecret,
    isPasswordValid, passwordsMatch,
    updatePlatform, toDto,
  };
});

function defaultInstallPath(): string {
  return "/opt/openclaw";
}
