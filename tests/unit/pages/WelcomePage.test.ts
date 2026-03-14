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
    mockInvoke.mockResolvedValue(null);
    const { pinia, router } = makeApp();
    const wrapper = mount(WelcomePage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.text()).toContain("OpenClaw");
  });

  it("检测到已有安装时显示三个操作选项", async () => {
    mockInvoke.mockResolvedValue({
      version: "1.0.0", install_path: "/opt/openclaw",
      installed_at: "2026-01-01", service_port: 18789,
    });
    const { pinia, router } = makeApp();
    const wrapper = mount(WelcomePage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.find(".existing-info").exists()).toBe(true);
    expect(wrapper.findAll(".mode-card")).toHaveLength(3);
    expect(wrapper.text()).toContain("1.0.0");
  });
});
