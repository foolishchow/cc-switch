/**
 * listen 轮询测试（A-007 / R-008）。
 *
 * 验证 shim-event 的 `listen` 在轮询周期内拉取 REST 端点，数据变化时触发 handler。
 * 用假时钟推进 + mock fetch 返回变化的数据。
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listen } from "../src/web/shim-event";

describe("listen polling (A-007)", () => {
  beforeEach(() => {
    localStorage.setItem("ccs.restToken", "test-token");
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("proxy-status-changed: 数据变化时回调", async () => {
    const seq = [{ running: false }, { running: true }];
    let i = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: true,
        status: 200,
        json: async () => seq[Math.min(i++, seq.length - 1)],
      })) as unknown as typeof fetch,
    );

    const calls: unknown[] = [];
    const unlisten = await listen("proxy-status-changed", (e) => calls.push(e.payload));

    // 初始周期（intervalMs=2000）后第一次拉取；此时 running=false → 首次触发。
    await vi.advanceTimersByTimeAsync(2010);
    // 第二周期 → running=true（变化）→ 触发。
    await vi.advanceTimersByTimeAsync(2010);

    unlisten();
    expect(calls.length).toBeGreaterThanOrEqual(2);
    expect(calls[0]).toEqual({ running: false });
    expect(calls[1]).toEqual({ running: true });
  });

  it("未在 POLL_MAP 的事件 → no-op unlisten，不抛", async () => {
    const un = await listen("some-desktop-only-event", () => {});
    expect(typeof un).toBe("function");
    un(); // 不抛
  });

  it("unlisten 后停止轮询", async () => {
    let n = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, status: 200, json: async () => ({ n: ++n }) }) as unknown as Response),
    );
    const calls: unknown[] = [];
    const un = await listen("proxy-status-changed", (e) => calls.push(e.payload));
    await vi.advanceTimersByTimeAsync(2010);
    const before = calls.length;
    un();
    await vi.advanceTimersByTimeAsync(6030);
    expect(calls.length).toBe(before); // unlisten 后不再回调
  });
});
