import { expect, test } from "@playwright/test";

for (const width of [1280, 390]) {
  test(`first-run restoration retries safely and opens the library at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 844 });
    await page.addInitScript(() => {
      let initialized = false, attempts = 0;
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === "dashboard") return { initialized, inventory: { skill: initialized ? 2 : 0 }, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/test/logs", canonical_root: "/test/hub", git_available: true, platform: "linux" };
        if (command === "inventory") return [];
        if (command === "initial_scan") throw new Error("Restore must never scan");
        if (command === "restore_library") {
          attempts++;
          if (args.branch !== "" || args.username !== "tester" || args.token !== "fixture-only") throw new Error("Incorrect restoration arguments");
          await new Promise((resolve) => setTimeout(resolve, 450));
          if (attempts === 1) throw new Error("Authentication failed: check your access token");
          initialized = true;
          return { imported: 2, branch: "agenthub", recovery_path: "/test/backup" };
        }
        if (command === "debug_event" || command === "open_token_settings") return;
        throw new Error(`Unexpected IPC: ${command}`);
      } } });
    });
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "我已经有 AgentHub 库" })).toBeVisible();
    await page.screenshot({ path: info.outputPath("first-run-choices.png"), animations: "disabled" });
    await page.getByRole("button", { name: "恢复已有 AgentHub 库", exact: true }).click();
    const repository = page.getByLabel("远程仓库地址");
    await expect(repository).toBeFocused();
    await repository.fill(width === 1280 ? "https://github.com/tester/library.git" : "https://git.example/team/library.git");
    await page.getByLabel("登录用户名").fill("tester");
    const token = page.getByLabel("访问令牌", { exact: true });
    await token.fill("fixture-only");
    await expect(token).toHaveAttribute("type", "password");
    await page.getByRole("button", { name: "显示访问令牌" }).click();
    await expect(token).toHaveAttribute("type", "text");
    await page.getByRole("button", { name: "隐藏访问令牌" }).click();
    const submit = page.getByRole("button", { name: "认证并恢复能力库" });
    await submit.click();
    await expect(submit).toBeDisabled();
    await expect(page.getByRole("button", { name: "返回选择" })).toBeDisabled();
    await expect(page.getByRole("alert")).toContainText("Authentication failed");
    await expect(page.getByRole("alert")).toBeFocused();
    await expect(token).toHaveValue("fixture-only");
    await page.screenshot({ path: info.outputPath("restore-retry.png"), animations: "disabled" });
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await token.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("heading", { name: "我的能力库" })).toBeVisible();
    await expect(page.getByRole("status").filter({ hasText: "项能力已从仓库恢复" })).toContainText("2 项能力已从仓库恢复");
    expect(await page.evaluate(() => JSON.stringify(localStorage).includes("fixture-only"))).toBe(false);
  });
}
