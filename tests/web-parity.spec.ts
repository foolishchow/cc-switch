/**
 * Parity spot-check（A-008）。
 *
 * 验证 `shim-core.invoke` 经 `routes-map` 派发后，返回形状 = Tauri command 返回形状。
 * 用 jsdom + mock fetch，断言：
 *  (1) 所有 48 命令都在 ROUTES 表内（无 UnsupportedInWebMode）；
 *  (2) 每个解包模式（ok / okNull / current / url / enabled / passthrough）的代表性命令，
 *      fetch 的 method + URL 正确，且 invoke 返回 = 预期 command 形状。
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, __setAutoRoutesForTest } from "../src/web/shim-core";
import type { RouteMeta } from "../src/web/shim-core";
import { ROUTES } from "../src/web/routes-map";

// routes-map 里登记的全部命令（mapping 完整性）。
const ALL_COMMANDS = [
  "get_providers","get_current_provider","switch_provider","add_provider","update_provider",
  "delete_provider","get_provider_editor_view","test_api_endpoints","get_custom_endpoints",
  "add_custom_endpoint","remove_custom_endpoint","update_providers_sort_order",
  "get_proxy_status","get_proxy_config_for_app","update_proxy_config_for_app",
  "get_proxy_takeover_status","set_proxy_takeover_for_app","get_provider_health",
  "get_circuit_breaker_config","update_circuit_breaker_config","get_circuit_breaker_stats",
  "reset_circuit_breaker","get_global_proxy_url","set_global_proxy_url","get_global_proxy_config",
  "update_global_proxy_config","scan_openclaw_config_health","get_failover_queue",
  "get_available_providers_for_failover","add_to_failover_queue","remove_from_failover_queue",
  "get_auto_failover_enabled","set_auto_failover_enabled","list_profiles","create_profile",
  "update_profile","delete_profile","apply_profile","clear_current_profile",
  "get_settings","save_settings",
  "get_config_dir","get_app_config_dir_override",
  "get_installed_skills","get_migration_result","get_skills_migration_result",
  "get_rest_config","get_pi_current_state",
] as const;

// 按路径前缀返回 canned REST 响应（模拟 REST handler 返回的形状）。
const CANNED: Record<string, unknown> = {
  "/control/v1/providers/switch": { warnings: [] },               // passthrough SwitchResult
  "/control/v1/providers/current": "claude-official",           // passthrough raw String (migrated, 信封已去)
  "/control/v1/providers": { ok: true },                          // unwrap ok (add) / okNull
  "/control/v1/settings": { ok: true },                               // save_settings unwrap ok / get passthrough
  "/control/v1/global-proxy/url": { url: "http://127.0.0.1:7890" }, // unwrap url
  "/control/v1/failover/auto": true,                             // passthrough raw bool (migrated, 信封已去)
  "/control/v1/proxy/status": { running: true, address: "127.0.0.1", port: 8787 }, // passthrough
  "/control/v1/profiles": { profiles: [], currentIds: { claude: null, claudeDesktop: null, codex: null } }, // passthrough
  "/control/v1/__routes": [],
  "/control/v1/auto_probe": [{ x: 1 }],
};

function cannedFor(url: string): unknown {
  const u = new URL(url, "http://x");
  // 长路径优先，避免 /providers 错配 /providers/switch。
  const keys = Object.keys(CANNED).sort((a, b) => b.length - a.length);
  for (const key of keys) {
    if (u.pathname === key || u.pathname.startsWith(key + "/")) return CANNED[key];
  }
  return { ok: true }; // mutation 默认
}

let lastCall: { method: string; url: string; body: unknown | undefined } | null = null;

beforeEach(() => {
  localStorage.setItem("ccs.restToken", "test-token");
  lastCall = null;
  __setAutoRoutesForTest(null);
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      lastCall = { method: init?.method ?? "GET", url, body: init?.body };
      return {
        ok: true,
        status: 200,
        json: async () => cannedFor(url as string),
      } as Response;
    }),
  );
});

describe("routes-map completeness", () => {
  it("所有 48 命令都在 ROUTES 内", () => {
    for (const cmd of ALL_COMMANDS) {
      expect(ROUTES[cmd], `missing mapping: ${cmd}`).toBeDefined();
    }
    expect(ALL_COMMANDS.length).toBeGreaterThanOrEqual(48);
  });
});

describe("unwrap parity", () => {
  it("current → bare String", async () => {
    const r = await invoke<string>("get_current_provider", { app: "claude" });
    expect(r).toBe("claude-official");
    expect(lastCall!.method).toBe("GET");
    expect(lastCall!.url).toContain("/control/v1/providers/current");
    expect(lastCall!.url).toContain("app=claude");
  });

  it("url → bare Option<String>", async () => {
    const r = await invoke<string | null>("get_global_proxy_url");
    expect(r).toBe("http://127.0.0.1:7890");
    expect(lastCall!.method).toBe("GET");
  });

  it("enabled → bare bool", async () => {
    const r = await invoke<boolean>("get_auto_failover_enabled", { appType: "claude" });
    expect(r).toBe(true);
    expect(lastCall!.url).toContain("/control/v1/failover/auto");
    expect(lastCall!.url).toContain("app=claude");
  });

  it("ok → bare bool (switch-ish mutation)", async () => {
    const r = await invoke<boolean>("add_provider", { app: "claude", provider: {} });
    expect(r).toBe(true);
    expect(lastCall!.method).toBe("POST");
    expect(lastCall!.url).toContain("/control/v1/providers");
  });

  it("okNull → null (() mutation)", async () => {
    const r = await invoke("add_custom_endpoint", { app: "claude", providerId: "x", url: "http://y" });
    expect(r).toBeNull();
    expect(lastCall!.method).toBe("POST");
    expect(lastCall!.url).toContain("/control/v1/providers/x/custom-endpoints");
  });

  it("passthrough SwitchResult", async () => {
    const r = await invoke("switch_provider", { app: "claude", id: "claude-official" });
    expect(r).toEqual({ warnings: [] });
    expect(lastCall!.method).toBe("POST");
    expect(lastCall!.body).toBe(JSON.stringify({ app: "claude", provider_id: "claude-official" }));
  });

  it("passthrough ProxyStatus", async () => {
    const r = await invoke("get_proxy_status");
    expect(r).toEqual({ running: true, address: "127.0.0.1", port: 8787 });
  });

  it("passthrough ProfilesResponse", async () => {
    const r = await invoke("list_profiles");
    expect(r).toEqual({
      profiles: [],
      currentIds: { claude: null, claudeDesktop: null, codex: null },
    });
  });
});

describe("arg routing", () => {
  it("delete_provider: id→path, app→query", async () => {
    await invoke("delete_provider", { app: "claude", id: "p1" });
    expect(lastCall!.method).toBe("DELETE");
    expect(lastCall!.url).toContain("/control/v1/providers/p1");
    expect(lastCall!.url).toContain("app=claude");
  });

  it("set_proxy_takeover_for_app: appType→path, enabled→query", async () => {
    await invoke("set_proxy_takeover_for_app", { appType: "claude", enabled: true });
    expect(lastCall!.method).toBe("POST");
    expect(lastCall!.url).toContain("/control/v1/proxy/takeover/claude");
    expect(lastCall!.url).toContain("enabled=true");
  });

  it("update_proxy_config_for_app: config body, path from config.appType", async () => {
    const config = { appType: "claude", enabled: true };
    await invoke("update_proxy_config_for_app", { config });
    expect(lastCall!.method).toBe("PUT");
    expect(lastCall!.url).toContain("/control/v1/proxy/config/claude");
    expect(lastCall!.body).toBe(JSON.stringify(config));
  });

  it("未映射命令 → 静默返回默认值（不抛）", async () => {
    // is_/has_ → false；含 dir/path/url → ""；其余默认 []（抗 .map/.length 崩）
    // 用真正未迁命令验证 defaultFor 降级（发现表也为空）
    expect(await invoke("get_model_pricing")).toEqual([]);
    expect(await invoke("get_unknown_log_dir")).toBe("");
  });

  it("自动路由：发现表命中 → 据元数据构造 POST body=args（passthrough）", async () => {
    const m = new Map<string, RouteMeta>([
      ["auto_probe", { cmd: "auto_probe", method: "POST", path: "/control/v1/auto_probe", pathParams: [] }],
    ]);
    __setAutoRoutesForTest(m);
    const r = (await invoke("auto_probe", { foo: "bar" })) as unknown;
    expect(r).toEqual([{ x: 1 }]);
    expect(lastCall!.method).toBe("POST");
    expect(lastCall!.url).toBe("/control/v1/auto_probe");
    expect(lastCall!.body).toBe(JSON.stringify({ foo: "bar" }));
    __setAutoRoutesForTest(null);
  });

  it("自动路由：GET + 标量查询参 → querystring（跳过对象/空）", async () => {
    const m = new Map<string, RouteMeta>([
      ["auto_get", { cmd: "auto_get", method: "GET", path: "/control/v1/auto_get", pathParams: [] }],
    ]);
    __setAutoRoutesForTest(m);
    await invoke("auto_get", { appType: "claude", filter: { a: 1 }, page: null });
    expect(lastCall!.method).toBe("GET");
    expect(lastCall!.url).toBe("/control/v1/auto_get?appType=claude");
    __setAutoRoutesForTest(null);
  });

  it("自动路由：path 参 → 填入 URL 并从 body 删除", async () => {
    const m = new Map<string, RouteMeta>([
      ["auto_del", { cmd: "auto_del", method: "DELETE", path: "/control/v1/items/:id", pathParams: ["id"] }],
    ]);
    __setAutoRoutesForTest(m);
    await invoke("auto_del", { id: 42, reason: "x" });
    expect(lastCall!.method).toBe("DELETE");
    expect(lastCall!.url).toBe("/control/v1/items/42");
    expect(lastCall!.body).toBe(JSON.stringify({ reason: "x" }));
    __setAutoRoutesForTest(null);
  });
});
