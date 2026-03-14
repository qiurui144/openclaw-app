import { defineStore } from "pinia";
import { ref, computed } from "vue";

export interface AiConfigDto {
  provider: string;
  base_url: string;
  api_key: string;
  model: string;
}

/** 企业微信自建应用凭据 */
export interface WecomConfigDto {
  corp_id: string;
  corp_secret: string;
  agent_id: string;
}

/** 钉钉应用凭据（AppKey + AppSecret，Stream 长连接） */
export interface DingtalkConfigDto {
  client_id: string;
  client_secret: string;
}

/** 飞书应用机器人凭据（App ID + App Secret，WebSocket 长连接） */
export interface FeishuConfigDto {
  app_id: string;
  app_secret: string;
}

/** QQ 开放平台机器人凭据（回调推送模式） */
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
  wecom_config: WecomConfigDto | null;
  dingtalk_config: DingtalkConfigDto | null;
  feishu_config: FeishuConfigDto | null;
  qq_config: QqConfigDto | null;
  ai_config: AiConfigDto | null;
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

  // 企业微信：自建应用，需要 corpId + corpSecret + agentId
  const wecomEnabled    = ref(false);
  const wecomCorpId     = ref("");
  const wecomCorpSecret = ref("");
  const wecomAgentId    = ref("");

  // 钉钉：Stream 长连接，clientId (AppKey) + clientSecret (AppSecret)
  const dingtalkEnabled      = ref(false);
  const dingtalkClientId     = ref("");
  const dingtalkClientSecret = ref("");

  // 飞书：应用机器人，App ID + App Secret（WebSocket 长连接，无需公网 IP）
  const feishuEnabled   = ref(false);
  const feishuAppId     = ref("");
  const feishuAppSecret = ref("");

  // QQ 机器人（AppID + AppSecret，回调推送模式）
  const qqEnabled   = ref(false);
  const qqAppId     = ref("");
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

  function toDto(): DeployConfigDto {
    return {
      install_path: installPath.value,
      service_port: servicePort.value,
      admin_password: adminPassword.value,
      domain_name: domainName.value,
      install_service: installService.value,
      start_on_boot: startOnBoot.value,
      source_mode: { type: "bundled" },
      wecom_config: wecomEnabled.value && wecomCorpId.value && wecomCorpSecret.value && wecomAgentId.value
        ? { corp_id: wecomCorpId.value, corp_secret: wecomCorpSecret.value, agent_id: wecomAgentId.value }
        : null,
      dingtalk_config: dingtalkEnabled.value && dingtalkClientId.value && dingtalkClientSecret.value
        ? { client_id: dingtalkClientId.value, client_secret: dingtalkClientSecret.value }
        : null,
      feishu_config: feishuEnabled.value && feishuAppId.value && feishuAppSecret.value
        ? { app_id: feishuAppId.value, app_secret: feishuAppSecret.value }
        : null,
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
    domainName, installService, startOnBoot, clashSubscriptionUrl, localZipPath,
    wecomEnabled, wecomCorpId, wecomCorpSecret, wecomAgentId,
    dingtalkEnabled, dingtalkClientId, dingtalkClientSecret,
    feishuEnabled, feishuAppId, feishuAppSecret,
    qqEnabled, qqAppId, qqAppSecret,
    aiProvider, aiBaseUrl, aiApiKey, aiModel,
    isPasswordValid, passwordsMatch,
    toDto,
  };
});

function defaultInstallPath(): string {
  return ""; // 由 InstallConfigPage 通过 get_default_install_path 命令填充
}
