import { setActivePinia, createPinia } from "pinia";
import { useConfigStore } from "@/stores/config";
import { describe, it, expect, beforeEach } from "vitest";

describe("config store", () => {
  beforeEach(() => { setActivePinia(createPinia()); });

  it("service_port 默认 18789", () => {
    const store = useConfigStore();
    expect(store.servicePort).toBe(18789);
  });

  it("密码在 toDto() 中正确传递", () => {
    const store = useConfigStore();
    store.adminPassword = "s3cret!";
    expect(store.toDto().admin_password).toBe("s3cret!");
  });

  it("未启用任何平台时 toDto() 所有平台配置为 null", () => {
    const store = useConfigStore();
    const dto = store.toDto();
    expect(dto.wecom_config).toBeNull();
    expect(dto.dingtalk_config).toBeNull();
    expect(dto.feishu_config).toBeNull();
    expect(dto.qq_config).toBeNull();
  });

  it("启用飞书并填写凭据后 toDto() 包含 feishu_config", () => {
    const store = useConfigStore();
    store.feishuEnabled = true;
    store.feishuAppId = "cli_test";
    store.feishuAppSecret = "fs_secret";
    const dto = store.toDto();
    expect(dto.feishu_config).toEqual({ app_id: "cli_test", app_secret: "fs_secret" });
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
