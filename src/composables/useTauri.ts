import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onUnmounted } from "vue";
import type { CheckItem, DeployProgress } from "@/stores/wizard";

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

export const tauri = {
  runSystemCheck: () => invoke<CheckItem[]>("run_system_check"),
  loadSession: () => invoke<{ install_path: string; source_mode: string } | null>("load_session"),
  clearSession: (installPath?: string) => invoke<void>("clear_session", { installPath }),
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
  getDefaultInstallPath: () => invoke<string>("get_default_install_path"),
  healthCheck: (port: number) => invoke<void>("health_check", { port }),
  runUninstall: (installPath: string) => invoke<void>("run_uninstall", { installPath }),
  // 服务控制（托盘 + FinishPage 共用）
  serviceStatus: () => invoke<"running" | "stopped" | "unknown">("service_status"),
  serviceStart: () => invoke<void>("service_start"),
  serviceStop: () => invoke<void>("service_stop"),
  notifyDeployDone: () => invoke<void>("notify_deploy_done"),
};

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
