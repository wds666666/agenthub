import { expect, test } from "@playwright/test";

for (const width of [1280, 390]) {
  test(`direct Skill folder import reviews payload and keeps operations separate at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 700 });
    await page.addInitScript(() => {
      let picks = 0;
      let imports = 0;
      let imported = false;
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === "dashboard") return { initialized: true, inventory: { skill: imported ? 1 : 0 }, enabled_targets: [], auto_sync_targets: [], dirty: imported, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/fixture/logs", canonical_root: "/fixture/hub", git_available: true, platform: "windows" };
        if (command === "check_skill_changes") return { changes: [], errors: [] };
        if (command === "inventory") return imported ? [{ id: "example", kind: "skill", display_name: "example", path: "/fixture/hub/skills/example", digest: "new", compatible_targets: ["agents"] }] : [];
        if (command === "pick_local_skill") {
          picks++;
          if (picks === 1) return null; // Native picker cancellation must not import.
          if (picks === 2) throw new Error("read SKILL.md: missing from folder root");
          return { id: `preview-${picks}`, item: { id: "source", kind: "skill", source: "local", path: "C:\\Downloads\\example", digest: "new", importable: picks !== 3, selected: false }, files: [{ path: "SKILL.md", size: 123 }, { path: "nested/SKILL.md", size: 100 }, ...Array.from({ length: 50 }, (_, index) => ({ path: `references/very-long-reference-folder-name/file-${index}.md`, size: 10 }))], excluded: [".venv", "node_modules"], duplicate_of: picks === 3 ? "existing-example" : null, canonical_digest: "library" };
        }
        if (command === "import_local_skill") {
          if (args.planId !== "preview-4" && args.planId !== "preview-5") throw new Error("Unreviewed Skill import");
          imports++;
          if (imports === 1) throw new Error("Skill source changed; choose the Skill again to refresh the preview");
          imported = true;
          return { imported: ["example"], skipped_duplicates: 0, auto_sync: [] };
        }
        if (command === "debug_event") return;
        // Discovery, version saves, remote operations and host writes are forbidden here.
        throw new Error(`Unexpected IPC ${command}`);
      } } });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "我的能力库", exact: true }).click();
    await expect(page.getByRole("button", { name: "扫描并导入", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "导入 Skill", exact: true }).click();
    const dialog = page.getByRole("dialog");
    const choose = dialog.getByRole("button", { name: "选择 Skill 文件夹" });
    const apply = dialog.getByRole("button", { name: "导入到 AgentHub" });
    await expect(dialog).toContainText("不自动保存版本、推送或同步到工具");
    await expect(apply).toBeDisabled();
    await choose.click();
    await expect(apply).toBeDisabled();
    await choose.click();
    await expect(dialog.getByRole("alert")).toContainText("missing from folder root");
    await expect(apply).toBeDisabled();
    await choose.click();
    await expect(dialog).toContainText("existing-example");
    await expect(apply).toBeDisabled();
    await choose.click();
    await expect(apply).toBeEnabled();
    await expect(dialog.getByText("nested/SKILL.md", { exact: true })).toBeVisible();
    await dialog.getByText("已排除的环境与缓存 · 2", { exact: true }).click();
    await dialog.getByText(".venv", { exact: true }).scrollIntoViewIfNeeded();
    await expect(apply).toBeInViewport();
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await page.screenshot({ path: info.outputPath("direct-skill-import.png"), animations: "disabled" });
    await apply.click();
    await expect(dialog.getByRole("alert")).toContainText("Skill source changed");
    await choose.click();
    await apply.click();
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole("button", { name: "查看详情 example" })).toBeVisible();
    await expect(page.getByRole("status").filter({ hasText: "项已导入" })).toContainText("1 项已导入");
  });
}
