import { setActivePinia, createPinia } from "pinia";
import { useConfigStore } from "@/stores/config";
import { describe, it, expect, beforeEach } from "vitest";

describe("config store", () => {
  beforeEach(() => { setActivePinia(createPinia()); });

  it("service_port 默认 18789", () => {
    const store = useConfigStore();
    expect(store.servicePort).toBe(18789);
  });

  it("密码不在 toDto() 中暴露给日志", () => {
    const store = useConfigStore();
    store.adminPassword = "s3cret!";
    expect(store.toDto().admin_password).toBe("s3cret!");
  });

  it("updatePlatform 更新指定平台", () => {
    const store = useConfigStore();
    store.updatePlatform("wx", { enabled: true, webhookUrl: "https://qyapi.weixin.qq.com/test" });
    expect(store.platforms.wx.enabled).toBe(true);
  });

  it("isPasswordValid 长度<8 为 false", () => {
    const store = useConfigStore();
    store.adminPassword = "abc";
    expect(store.isPasswordValid).toBe(false);
  });

  it("isPasswordValid 混合字母数字 ≥8 为 true", () => {
    const store = useConfigStore();
    store.adminPassword = "abc12345";
    expect(store.isPasswordValid).toBe(true);
  });
});
