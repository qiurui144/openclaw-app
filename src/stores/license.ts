import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type LicensePlan = "free" | "pro_single" | "pro_all" | "enterprise";
export type AuthMode = "free" | "code" | "payment";

export interface LicenseStatus {
  authenticated: boolean;
  plan: LicensePlan;
  auth_mode: AuthMode;
  user_id: string | null;
  skills: string[];
  expires_at: string | null;
  in_grace_period: boolean;
  device_bound: boolean;
}

export interface SkillEntry {
  slug: string;
  name: string;
  description: string;
  category: string;
  is_paid: boolean;
  price: number | null;
  price_label: string | null;
  version: string;
  author: string;
  icon: string | null;
}

export interface PaymentOrder {
  order_id: string;
  qr_url: string;
  amount: number;
  status: string;
}

export const useLicenseStore = defineStore("license", () => {
  const status = ref<LicenseStatus>({
    authenticated: false,
    plan: "free",
    auth_mode: "free",
    user_id: null,
    skills: [],
    expires_at: null,
    in_grace_period: false,
    device_bound: false,
  });

  const skillIndex = ref<SkillEntry[]>([]);
  const installedPaidSlugs = ref<string[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const isAuthenticated = computed(() => status.value.authenticated);
  const isPro = computed(() => ["pro_single", "pro_all", "enterprise"].includes(status.value.plan));

  const planLabel = computed(() => ({
    free: "免费版",
    pro_single: "Pro 单包",
    pro_all: "Pro 全包",
    enterprise: "企业版",
  }[status.value.plan]));

  const freeSkills = computed(() => skillIndex.value.filter((s) => !s.is_paid));
  const paidSkills = computed(() => skillIndex.value.filter((s) => s.is_paid));

  function canAccessSkill(slug: string): boolean {
    if (!status.value.authenticated) return false;
    const plan = status.value.plan;
    if (plan === "pro_all" || plan === "enterprise") return true;
    if (plan === "pro_single") {
      return status.value.skills.includes("*") || status.value.skills.includes(slug);
    }
    return false;
  }

  async function loadStatus() {
    try {
      status.value = await invoke<LicenseStatus>("get_license_status");
    } catch {
      status.value = { authenticated: false, plan: "free", auth_mode: "free", user_id: null, skills: [], expires_at: null, in_grace_period: false, device_bound: false };
    }
  }

  async function sendCode(phone: string) {
    error.value = null;
    await invoke<void>("send_login_code", { phone });
  }

  async function login(phone: string, code: string) {
    error.value = null;
    loading.value = true;
    try {
      status.value = await invoke<LicenseStatus>("license_login", { phone, code });
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function redeemCode(code: string) {
    error.value = null;
    loading.value = true;
    try {
      status.value = await invoke<LicenseStatus>("redeem_activation_code", { code });
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function logout() {
    await invoke<void>("license_logout");
    status.value = { authenticated: false, plan: "free", auth_mode: "free", user_id: null, skills: [], expires_at: null, in_grace_period: false, device_bound: false };
  }

  async function refresh() {
    try {
      status.value = await invoke<LicenseStatus>("refresh_license");
    } catch { /* 静默失败 */ }
  }

  async function loadSkillIndex() {
    loading.value = true;
    try {
      skillIndex.value = await invoke<SkillEntry[]>("fetch_skill_index");
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  async function loadInstalledPaid() {
    installedPaidSlugs.value = await invoke<string[]>("list_paid_skills");
  }

  async function installPaidSkill(installPath: string, slug: string) {
    await invoke<void>("install_paid_skill", { installPath, slug });
    await loadInstalledPaid();
  }

  async function uninstallPaidSkill(slug: string) {
    await invoke<void>("uninstall_paid_skill", { slug });
    await loadInstalledPaid();
  }

  async function createPayment(plan: string, skillSlug?: string): Promise<PaymentOrder> {
    return invoke<PaymentOrder>("create_payment", { plan, skillSlug: skillSlug ?? null });
  }

  async function checkPayment(orderId: string): Promise<string> {
    return invoke<string>("check_payment", { orderId });
  }

  async function refreshExpiredSkills(installPath: string): Promise<string[]> {
    return invoke<string[]>("refresh_expired_skills", { installPath });
  }

  return {
    status, skillIndex, installedPaidSlugs, loading, error,
    isAuthenticated, isPro, planLabel, freeSkills, paidSkills,
    canAccessSkill, loadStatus, sendCode, login, redeemCode, logout, refresh,
    loadSkillIndex, loadInstalledPaid, installPaidSkill, uninstallPaidSkill,
    createPayment, checkPayment, refreshExpiredSkills,
  };
});
