import { describe, it, expect, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { tauri } from "@/composables/useTauri";

const mockInvoke = vi.mocked(invoke);

describe("useTauri", () => {
  it("runSystemCheck 调用正确的 command", async () => {
    mockInvoke.mockResolvedValue([]);
    await tauri.runSystemCheck();
    expect(mockInvoke).toHaveBeenCalledWith("run_system_check");
  });

  it("clashTest 传递 subscriptionUrl 参数", async () => {
    mockInvoke.mockResolvedValue({ success: true, latency_ms: 100, error: null });
    await tauri.clashTest("https://example.com/sub");
    expect(mockInvoke).toHaveBeenCalledWith("clash_test", { subscriptionUrl: "https://example.com/sub" });
  });

  it("startDeploy 传递 config 参数", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await tauri.startDeploy({ install_path: "/opt/openclaw" });
    expect(mockInvoke).toHaveBeenCalledWith("start_deploy", { config: { install_path: "/opt/openclaw" } });
  });
});
