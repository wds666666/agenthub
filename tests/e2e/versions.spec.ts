import { expect, test } from "@playwright/test";
for (const width of [1280, 390]) {
  test(`reviews capability changes and typed library recovery at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 844 });
    await page.addInitScript(() => {
      let attempted = 0;
      let dirty = true;
      const changes = [{ kind: "skill", id: "lark", action: "update", files: ["skills/lark/SKILL.md", "skills/lark/reference.md"] }, { kind: "mcp", id: "context7", action: "delete", files: ["mcp/context7/server.json"] }];
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === "dashboard") return { initialized: true, inventory: { skill: 1 }, enabled_targets: [], auto_sync_targets: [], dirty, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/fixture/logs", canonical_root: "/hub", git_available: true, platform: "linux" };
        if (command === "check_skill_changes") return { changes: [], errors: [] };
        if (command === "git_status") return dirty ? "## main\n M skills/lark/SKILL.md" : "## main";
        if (command === "git_diff") return dirty ? "fixture raw diff" : "";
        if (command === "git_changes") return dirty ? changes : [];
        if (command === "git_identity") return { name: "Fixture", email: "fixture@example.com" };
        if (command === "git_log") return "abc\t2026-10-05\tSaved library";
        if (command === "remote_settings") return { url: "https://git.example/library.git", branch: "main", state: "read_verified" };
        if (command === "version_recovery_plan") return { id: "reviewed-id", action: args.action, source_commit: "abcdef123456", changes };
        if (command === "version_recovery_apply") {
          if (args.planId !== "reviewed-id" || args.confirmation !== "DISCARD") throw new Error("Unchecked recovery");
          attempted++;
          if (attempted === 1) throw new Error("fixture recovery stopped; local edits retained");
          dirty = false;
          return { backup_path: "/hub/backups/library-version-fixture", source_commit: "abcdef123456", pending_changes: false };
        }
        if (command === "debug_event") return;
        throw new Error(`Unexpected IPC ${command}`);
      } } });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "版本记录", exact: true }).click();
    await expect(page.getByRole("heading", { name: "能力变更", exact: true })).toBeVisible();
    await expect(page.locator(".version-capability-changes").getByText("lark", { exact: true })).toBeVisible();
    expect((await page.locator(".version-changes-heading > div").boundingBox())?.width).toBeGreaterThan(100);
    await page.screenshot({ path: info.outputPath("version-changes.png"), fullPage: true, animations: "disabled" });
    await page.getByRole("button", { name: "放弃未保存改动", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("lark", { exact: true })).toBeVisible();
    const apply = dialog.getByRole("button", { name: "放弃未保存改动", exact: true });
    await expect(apply).toBeDisabled();
    await dialog.getByLabel("输入确认词 DISCARD").fill("DISCARD");
    await apply.click();
    await expect(dialog.getByRole("alert")).toContainText("local edits retained");
    await expect(dialog.getByLabel("输入确认词 DISCARD")).toHaveValue("DISCARD");
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await page.screenshot({ path: info.outputPath("version-recovery.png"), fullPage: true, animations: "disabled" });
    await apply.click();
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole("status").filter({ hasText: "/hub/backups/library-version-fixture" })).toContainText("/hub/backups/library-version-fixture");
    await expect(page.getByRole("button", { name: "放弃未保存改动", exact: true })).toBeDisabled();
    await page.getByRole("button", { name: "使用云端内容", exact: true }).click();
    await expect(dialog.getByLabel("输入确认词 REMOTE")).toBeVisible();
    await expect(dialog).toContainText("版本历史、设置和凭据保留");
  });
}

for (const width of [1280, 390]) {
  test(`remote recovery keeps long changes scrollable and actions reachable at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 600 });
    await page.addInitScript(() => {
      const changes = Array.from({ length: 40 }, (_, index) => ({ kind: "skill", id: `skill-${index}`, action: "update", files: Array.from({ length: index === 0 ? 544 : 2 }, (_, n) => `skills/skill-${index}/references/file-${n}.md`) }));
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
        if (command === "dashboard") return { initialized: true, inventory: { skill: 40 }, enabled_targets: [], auto_sync_targets: [], dirty: true, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/fixture/logs", canonical_root: "/hub", git_available: true, platform: "windows" };
        if (command === "check_skill_changes") return { changes: [], errors: [] };
        if (command === "git_status") return "## agenthub\n A mcp/server/server.json";
        if (command === "git_diff") return "";
        if (command === "git_changes") return [];
        if (command === "git_identity") return { name: "Fixture", email: "fixture@example.com" };
        if (command === "git_log") return "abc\t2026-10-07\tSaved library";
        if (command === "remote_settings") return { url: "https://git.example/library.git", branch: "agenthub", state: "read_verified" };
        if (command === "version_recovery_plan") return { id: "long-plan", action: "remote", source_commit: "abcdef", changes };
        if (command === "debug_event") return;
        throw new Error(`Unexpected IPC ${command}`);
      } } });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "版本记录", exact: true }).click();
    await page.getByRole("button", { name: "使用云端内容", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByRole("heading", { name: "使用云端内容", exact: true })).toBeVisible();
    const bounds = await dialog.boundingBox();
    expect(bounds!.y).toBeGreaterThanOrEqual(0);
    expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(600);
    const body = dialog.locator(".dialog-body");
    expect(await body.evaluate(node => node.scrollHeight > node.clientHeight)).toBe(true);
    await dialog.locator("summary").first().click();
    await body.hover();
    await page.mouse.wheel(0, 500);
    await expect.poll(() => body.evaluate(node => node.scrollTop)).toBeGreaterThan(0);
    await dialog.getByLabel("输入确认词 REMOTE").fill("REMOTE");
    const inputBounds = await dialog.getByLabel("输入确认词 REMOTE").boundingBox();
    const bodyBounds = await body.boundingBox();
    expect(inputBounds!.y + inputBounds!.height).toBeLessThanOrEqual(bodyBounds!.y + bodyBounds!.height + 1);
    await expect(dialog.getByRole("button", { name: "使用云端内容", exact: true })).toBeEnabled();
    const footer = await dialog.locator("footer").boundingBox();
    expect(footer!.y + footer!.height).toBeLessThanOrEqual(600);
    await expect(dialog.getByText("skill-39", { exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await page.screenshot({ path: info.outputPath("long-remote-recovery.png"), animations: "disabled" });
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole("button", { name: "使用云端内容", exact: true })).toBeFocused();
  });
}
