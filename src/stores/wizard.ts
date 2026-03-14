import { defineStore } from "pinia";
import { ref } from "vue";

export type CheckStatus = "pending" | "running" | "ok" | "warn" | "error";
export type DeployStatus = "idle" | "running" | "done" | "failed";
export type SourceMode = "bundled" | "online" | "local_zip";
export type WizardMode = "install" | "update" | "uninstall";

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
  const wizardMode = ref<WizardMode>("install");
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
  function setWizardMode(m: WizardMode) { wizardMode.value = m; }
  function setChecks(items: CheckItem[]) { systemChecks.value = items; }
  function setSourceMode(m: SourceMode) { sourceMode.value = m; }
  function setClashAccepted(v: boolean) { clashAccepted.value = v; }
  function setDeployStatus(s: DeployStatus) { deployStatus.value = s; }
  function updateProgress(p: DeployProgress) {
    deployProgress.value = p;
    deployLogs.value.push(`[${new Date().toLocaleTimeString()}] ${p.message}`);
  }
  function appendLog(line: string) {
    deployLogs.value.push(line);
  }
  function setExistingInstall(version: string, path: string) {
    isExistingInstall.value = true;
    existingVersion.value = version;
    existingPath.value = path;
  }

  return {
    currentPage, wizardMode, canProceed, systemChecks, sourceMode,
    clashAccepted, deployStatus, deployProgress, deployLogs,
    isExistingInstall, existingVersion, existingPath,
    setReady, setWizardMode, setChecks, setSourceMode, setClashAccepted,
    setDeployStatus, updateProgress, appendLog, setExistingInstall,
  };
});
