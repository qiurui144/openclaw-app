import { mount, flushPromises } from "@vue/test-utils";
import DeploymentPage from "@/pages/DeploymentPage.vue";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

describe("DeploymentPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
    mockListen.mockResolvedValue(() => {});
  });

  it("挂载后调用 start_deploy", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({ history: createWebHistory(), routes: [
      { path: "/deploy", name: "deploy", component: DeploymentPage },
      { path: "/finish", name: "finish", component: { template: "<div/>" } },
    ]});
    await router.push("/deploy");
    mount(DeploymentPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("start_deploy", expect.anything());
  });

  it("进度条初始为 0%", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: DeploymentPage }] });
    const wrapper = mount(DeploymentPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.find(".progress-fill").attributes("style")).toContain("0%");
  });
});
