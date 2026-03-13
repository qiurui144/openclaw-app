import { mount, flushPromises } from "@vue/test-utils";
import ClashDisclaimerPage from "@/pages/ClashDisclaimerPage.vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import { vi, describe, it, expect, beforeEach } from "vitest";

describe("ClashDisclaimerPage", () => {
  it("未勾选时 checkbox 未选中", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ClashDisclaimerPage }] });
    const wrapper = mount(ClashDisclaimerPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    expect(wrapper.find('input[type="checkbox"]').element.checked).toBe(false);
  });

  it("勾选后 checkbox 为选中状态", async () => {
    const pinia = createPinia();
    const router = createRouter({ history: createWebHistory(), routes: [{ path: "/", component: ClashDisclaimerPage }] });
    const wrapper = mount(ClashDisclaimerPage, { global: { plugins: [pinia, router] } });
    await flushPromises();
    await wrapper.find('input[type="checkbox"]').setValue(true);
    expect(wrapper.find('input[type="checkbox"]').element.checked).toBe(true);
  });
});
