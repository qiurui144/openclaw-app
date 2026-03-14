import { mount, flushPromises } from "@vue/test-utils";
import ServiceConfigPage from "@/pages/ServiceConfigPage.vue";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

describe("ServiceConfigPage", () => {
  beforeEach(() => { vi.clearAllMocks(); mockInvoke.mockResolvedValue({}); });

  it("端口超出范围时 canProceed 为 false", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ServiceConfigPage }] });
    const wrapper = mount(ServiceConfigPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    await wrapper.find('input[type="number"]').setValue(80);
    await flushPromises();
    const { useWizardStore } = await import("@/stores/wizard");
    const w = useWizardStore();
    expect(w.canProceed).toBe(false);
  });

  it("密码不匹配时 canProceed 为 false", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ServiceConfigPage }] });
    const wrapper = mount(ServiceConfigPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    const inputs = wrapper.findAll('input[type="password"]');
    await inputs[0].setValue("abc12345");
    // 确认密码不填，不匹配
    await flushPromises();
    const { useWizardStore } = await import("@/stores/wizard");
    const w = useWizardStore();
    expect(w.canProceed).toBe(false);
  });
});
