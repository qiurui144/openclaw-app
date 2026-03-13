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
  source_mode: { type: string; proxy_url?: string; path?: string };
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
      source_mode: { type: "bundled" },
      platforms: platforms.value,
    };
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
  return "/opt/openclaw";
}
