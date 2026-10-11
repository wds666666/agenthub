import { test, expect } from "@playwright/test";

for (const width of [1280, 390]) {
  test(`background settings, refresh and close choices at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 844 });
    await page.addInitScript(() => {
      let settings = { close_behavior: "ask", check_seconds: 120, sync_seconds: 0, paused: false };
      const status = { running: false, pending: false, tray_available: true, watcher_available: true };
      const calls: Array<{ command: string; args: unknown }> = [];
      const callbacks = new Map<number, (event: unknown) => void>();
      const listeners = new Map<string, number>(); let id = 0;
      Object.defineProperty(window, "backgroundFixture", { value: { calls, status, emit: (name: string) => callbacks.get(listeners.get(name)!)?.({ payload: null }) } });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", { value: { unregisterListener: () => undefined } });
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {
        transformCallback: (callback: (event: unknown) => void) => { callbacks.set(++id, callback); return id; },
        invoke: async (command: string, args: Record<string, unknown>) => {
          if (command === "plugin:event|listen") { listeners.set(args.event as string, args.handler as number); return args.handler; }
          if (command === "plugin:event|unlisten") return;
          if (command === "dashboard") return { initialized: true, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
          if (command === "runtime_diagnostics") return { log_dir: "/logs", canonical_root: "/hub", git_available: true, platform: "linux" };
          if (command === "check_skill_changes") return { changes: [], errors: [] };
          if (command === "background_settings") return settings;
          if (command === "background_status") return status;
          if (command === "policy_settings") return { sync_mode: "preserve", strict_authoritative: false, sync_after_reverse_import: false };
          if (command === "debug_event") return;
          calls.push({ command, args });
          if (command === "set_background_settings") { settings = args.settings as typeof settings; return settings; }
          if (command === "background_refresh" || command === "close_choice") return;
          throw new Error(command);
        },
      } });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "设置与诊断", exact: true }).click();
    await page.getByLabel("工具变更检查频率").selectOption("300");
    await page.getByLabel("库到工具的同步时机").selectOption("60");
    await page.getByLabel("暂停自动同步", { exact: true }).check();
    await page.getByRole("button", { name: "保存后台设置" }).click();
    await expect(page.getByRole("status").filter({ hasText: "后台设置已保存" })).toBeVisible();
    await page.getByRole("button", { name: "立即检查并同步", exact: true }).click();
    await expect(page.getByText("已请求检查与同步", { exact: true })).toBeVisible();
    expect(await page.evaluate(() => (window as unknown as { backgroundFixture: { calls: unknown[] } }).backgroundFixture.calls)).toEqual([
      { command: "set_background_settings", args: { settings: { close_behavior: "ask", check_seconds: 300, sync_seconds: 60, paused: true } } },
      { command: "background_refresh", args: {} },
    ]);
    await page.evaluate(() => (window as unknown as { backgroundFixture: { emit: (s: string) => void } }).backgroundFixture.emit("close-requested"));
    await expect(page.getByRole("dialog", { name: "关闭 AgentHub" })).toBeVisible();
    await page.getByRole("button", { name: "留在系统托盘", exact: true }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await page.evaluate(() => { const f = (window as unknown as { backgroundFixture: { status: { tray_available: boolean }; emit: (s: string) => void } }).backgroundFixture; f.status.tray_available = false; f.emit("close-requested"); });
    await expect(page.getByRole("button", { name: "留在系统托盘", exact: true })).toBeDisabled();
    await expect(page.getByText("当前桌面未提供可用系统托盘，关闭时会保留窗口并提示。", { exact: true }).last()).toBeVisible();
    await page.getByRole("button", { name: "继续使用", exact: true }).click();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
    await page.screenshot({ path: info.outputPath(`background-${width}.png`), fullPage: true });
  });
}
