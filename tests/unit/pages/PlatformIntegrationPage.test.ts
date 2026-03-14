import { mount, flushPromises } from "@vue/test-utils";
import PlatformIntegrationPage from "@/pages/PlatformIntegrationPage.vue";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { describe, it, expect, vi } from "vitest";

describe("PlatformIntegrationPage", () => {
  it("页面加载时 canProceed 为 true（可选页面）", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: PlatformIntegrationPage }] });
    mount(PlatformIntegrationPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    const { useWizardStore } = await import("@/stores/wizard");
    const w = useWizardStore();
    expect(w.canProceed).toBe(true);
  });

  it("显示 4 个平台（企业微信/钉钉/飞书/QQ）", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: PlatformIntegrationPage }] });
    const wrapper = mount(PlatformIntegrationPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.findAll(".platform-card")).toHaveLength(4);
  });
});
