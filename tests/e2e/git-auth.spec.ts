import { expect, test } from "@playwright/test";

for (const viewport of [{ width: 1280, height: 800 }, { width: 390, height: 844 }]) {
  test(`repository login, failed authentication and explicit retry at ${viewport.width}`, async ({ page }, testInfo) => {
    await page.setViewportSize(viewport);
    const repository = viewport.width === 1280 ? "https://github.com/tester/skills.git" : "https://git.example:20110/team/skills.git";
    await page.addInitScript((repository) => {
      let state = "unverified", saved = false, loginAttempts = 0, syncAttempts = 0;
      const remote = () => ({ url: repository, branch: "agenthub", state, credential_saved: saved });
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
        if (command === "dashboard") return { initialized: true, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/test/logs", canonical_root: "/test/hub", git_available: true, platform: "linux" };
        if (command === "git_status") return "## agenthub";
        if (command === "git_diff") return "";
        if (command === "git_log") return "abc123\t2026-10-04\tSaved skills";
        if (command === "git_identity") return { name: "Tester", email: "test@example.com" };
        if (command === "remote_settings") return remote();
        if (command === "login_remote") {
          loginAttempts++;
          await new Promise((resolve) => setTimeout(resolve, 400));
          if (loginAttempts === 1) { state = "auth_failed"; throw new Error("Authentication failed"); }
          state = "read_verified"; saved = true; return remote();
        }
        if (command === "sync_remote") { syncAttempts++; if (syncAttempts === 1) { state = "auth_failed"; throw new Error("Authentication failed"); } state = "synced"; return; }
        if (command === "forget_remote_credentials") { saved = false; state = "unverified"; return; }
        if (command === "debug_event" || command === "open_token_settings") return;
        throw new Error(`Unexpected IPC: ${command}`);
      } } });
    }, repository);
    await page.goto("/");
    await page.getByRole("button", { name: "版本记录", exact: true }).click();
    await expect(page.getByText("仓库已配置，尚未验证")).toBeVisible();
    await page.getByRole("button", { name: "仓库登录", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByRole("button", { name: viewport.width === 1280 ? "GitHub" : "自建 Git / Gitea", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("登录用户名")).toBeFocused();
    await dialog.getByRole("button", { name: "打开令牌管理页面", exact: true }).click();
    await page.getByLabel("登录用户名").fill("tester");
    const token = page.getByLabel("访问令牌", { exact: true });
    await token.fill("fixture-only-token");
    await expect(token).toHaveAttribute("type", "password");
    await page.getByRole("button", { name: "显示访问令牌" }).click();
    await expect(token).toHaveAttribute("type", "text");
    await page.getByRole("button", { name: "隐藏访问令牌" }).click();
    await page.getByRole("button", { name: "验证并保存登录" }).click();
    await expect(page.getByRole("button", { name: "正在验证…" })).toBeDisabled();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("alert")).toContainText("Authentication failed");
    await expect(token).toHaveValue("fixture-only-token");
    await page.screenshot({ path: testInfo.outputPath("git-login-failure.png") });
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await page.getByRole("button", { name: "验证并保存登录" }).click();
    await expect(dialog).toHaveCount(0);
    await expect(page.getByText("上次验证可读取")).toBeVisible();
    await page.getByRole("button", { name: "同步远端", exact: true }).click();
    await expect(dialog).toBeVisible();
    await expect(page.getByRole("button", { name: "登录并重试同步", exact: true }).last()).toBeVisible();
    await expect(token).toHaveValue("");
    await token.fill("fixture-only-token");
    await dialog.getByRole("button", { name: "登录并重试同步", exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(page.getByText("上次同步成功")).toBeVisible();
    await page.getByRole("button", { name: "删除保存的凭据", exact: true }).click();
    await expect(page.getByText("仓库已配置，尚未验证")).toBeVisible();
    await expect(page.getByText("已在本机加密保存登录凭据")).toHaveCount(0);
    expect(await page.evaluate(() => JSON.stringify(localStorage).includes("fixture-only-token"))).toBe(false);
  });
}
