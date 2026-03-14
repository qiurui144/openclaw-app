import { setActivePinia, createPinia } from "pinia";
import { useWizardStore } from "@/stores/wizard";
import { describe, it, expect, beforeEach } from "vitest";

describe("wizard store", () => {
  beforeEach(() => { setActivePinia(createPinia()); });

  it("初始页面为 welcome", () => {
    const store = useWizardStore();
    expect(store.currentPage).toBe("welcome");
  });

  it("canProceed 默认 false", () => {
    const store = useWizardStore();
    expect(store.canProceed).toBe(false);
  });

  it("setReady 后 canProceed 为 true", () => {
    const store = useWizardStore();
    store.setReady(true);
    expect(store.canProceed).toBe(true);
  });

  it("setChecks 更新 systemChecks", () => {
    const store = useWizardStore();
    store.setChecks([{ id: "os", label: "操作系统", required: true, status: "ok", detail: "" }]);
    expect(store.systemChecks).toHaveLength(1);
  });

  it("deployStatus 初始为 idle", () => {
    const store = useWizardStore();
    expect(store.deployStatus).toBe("idle");
  });
});
