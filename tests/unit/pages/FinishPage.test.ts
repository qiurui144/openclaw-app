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
    w.setDeployStatus("failed");
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: FinishPage }] });
    const wrapper = mount(FinishPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.find(".warn-banner").exists()).toBe(true);
  });
});
