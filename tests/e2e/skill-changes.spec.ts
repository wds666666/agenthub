import { expect, test } from "@playwright/test";

for (const width of [1280, 390]) {
  test(`Skill reminders review and import without manual discovery at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 844 });
    await page.addInitScript(() => {
      let imported = false;
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === "dashboard") return { initialized: true, inventory: { skill: 1 }, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/fixture/logs", canonical_root: "/fixture/hub", git_available: true, platform: "linux" };
        if (command === "inventory") return [{ id: "example", kind: "skill", display_name: "Example Skill", path: "/fixture/hub/skills/example", digest: "original", compatible_targets: ["agents"] }];
        if (command === "check_skill_changes") {
          await new Promise((resolve) => setTimeout(resolve, 200));
          return { changes: imported ? [] : ["agents", "cursor", "codex", "claude"].map((source) => ({ canonical_id: "example", item: { id: source, kind: "skill", source, path: `/fixture/.${source}/skills/example`, digest: "changed", selected: false, importable: true } })), errors: [] };
        }
        if (command === "initial_scan") throw new Error("Reminder review must not trigger a whole import scan");
        if (command === "import_scanned") {
          if (JSON.stringify(args.selectedIds) !== '["agents"]') throw new Error("Unexpected selection");
          imported = true;
          return { imported: ["example-2"], skipped_duplicates: 0, auto_sync: [] };
        }
        if (command === "debug_event") return;
        throw new Error(`Unexpected IPC ${command}`);
      } } });
    });
    await page.goto("/");
    await page.getByRole("button", { name: /我的能力库 · 项工具侧 Skill 内容有变化 4/ }).click();
    const reminder = page.getByRole("button", { name: "查看 Skill 变化 Example Skill" });
    await expect(reminder).toBeVisible();
    await expect(page.locator("nav .skill-change-dot")).toBeVisible();
    await page.screenshot({ path: info.outputPath("skill-reminder.png"), animations: "disabled" });
    await reminder.focus(); await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog");
    await expect(dialog).toContainText("原项会保留");
    const submit = dialog.getByRole("button", { name: "导入所选" });
    await expect(submit).toBeDisabled();
    await dialog.getByRole("checkbox", { name: /example.*agents/ }).check();
    await page.screenshot({ path: info.outputPath("skill-change-review.png"), animations: "disabled" });
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await submit.click();
    await expect(dialog).not.toBeVisible();
    await expect(reminder).not.toBeVisible();
    await expect(page.locator("nav .skill-change-dot")).not.toBeVisible();
  });
}
