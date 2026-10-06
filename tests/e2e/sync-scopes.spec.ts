import { expect, test } from "@playwright/test";

for (const width of [1280, 390]) {
  test(`preserves scope and selects individual Rules at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 844 });
    await page.addInitScript(() => {
      const items = [
        { id: "skill-one", kind: "skill", display_name: "One Skill", digest: "skill", path: "/hub/skills/skill-one" },
        { id: "rule-one", kind: "rule", display_name: "One Rule", digest: "one", path: "/hub/rules/rule-one" },
        { id: "rule-two", kind: "rule", display_name: "Two Rule", digest: "two", path: "/hub/rules/rule-two" },
      ];
      let attempts = 0;
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === "dashboard") return { initialized: true, inventory: { skill: 1, rule: 2 }, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/fixture/logs", canonical_root: "/hub", git_available: true, platform: "linux" };
        if (command === "check_skill_changes") return { changes: [], errors: [] };
        if (command === "inventory") return items.map((item) => ({ ...item, compatible_targets: ["codex", "cursor", "claude"] }));
        if (command === "policy_settings") return { sync_mode: "preserve", strict_authoritative: false, sync_after_reverse_import: false };
        if (command === "auto_sync_profiles") return [{ target: "codex", enabled: false, needs_review: true, selection: { mode: "preserve", skills_managed: true, skills: ["skill-one"], mcp_managed: false, mcp: [], plugins_managed: false, plugins: [], rules_managed: true, rule_ids: ["rule-one"], rules: false, authoritative: false } }];
        if (command === "create_plan") {
          const selection = args.selection as { rule_ids: string[]; mode: string };
          if (selection.mode !== "replace" || JSON.stringify(selection.rule_ids) !== '["rule-two"]') throw new Error("Scope unexpectedly expanded");
          attempts++;
          if (attempts === 1) throw new Error("fixture: target changed; retry preview");
          return { id: "plan", target: "codex", selection, steps: [], removed_resources: [{ kind: "mcp", id: "host-only-server" }], summary: [], warnings: [], canonical_digest: "abc", git: { dirty: false } };
        }
        if (command === "set_auto_sync") {
          if (args.reviewedPlanId !== "plan") throw new Error("Replacement was not reviewed");
          return { profile: { target: "codex", enabled: true, needs_review: false, selection: args.selection }, initial_sync: { changed: false } };
        }
        if (command === "debug_event") return;
        throw new Error(`Unexpected IPC ${command}`);
      } } });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "同步到工具", exact: true }).click();
    await expect(page.getByText("旧自动同步已暂停。请确认同步模式及逐条规则范围，再重新开启。")).toBeVisible();
    await expect(page.getByRole("radio", { name: /保留工具/ })).toBeChecked();
    const rules = page.locator("details").filter({ has: page.locator("summary", { hasText: "Rules" }) });
    await rules.getByRole("checkbox", { name: /One Rule/ }).uncheck();
    await rules.getByRole("checkbox", { name: /Two Rule/ }).check();
    await page.getByRole("radio", { name: /替换所选类别/ }).check();
    const preview = page.getByRole("button", { name: "预览变更", exact: true });
    await preview.click();
    await expect(page.getByText(/fixture: target changed; retry preview/)).toBeVisible();
    await expect(rules.getByRole("checkbox", { name: /Two Rule/ })).toBeChecked();
    await preview.click();
    await expect(page.getByText("变更预览已就绪", { exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
    await page.screenshot({ path: info.outputPath("selected-rule-mode.png"), fullPage: true, animations: "disabled" });
    await page.getByRole("button", { name: "开启自动同步", exact: true }).click();
    const confirmation = page.getByRole("dialog");
    await expect(confirmation.getByText("host-only-server", { exact: true })).toBeVisible();
    await confirmation.getByRole("button", { name: "同步当前范围并开启", exact: true }).click();
    await expect(confirmation).not.toBeVisible();
    await expect(page.getByText("旧自动同步已暂停。请确认同步模式及逐条规则范围，再重新开启。")).not.toBeVisible();

  });

  test(`MCP with different wrapper hashes is blocked as duplicate at ${width}`, async ({ page }) => {
    await page.setViewportSize({ width, height: 844 });
    await page.addInitScript(() => {
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
        if (command === "dashboard") return { initialized: true, inventory: { mcp: 1 }, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
        if (command === "runtime_diagnostics") return { log_dir: "/fixture/logs", canonical_root: "/hub", git_available: true, platform: "linux" };
        if (command === "check_skill_changes") return { changes: [], errors: [] };
        if (command === "inventory") return [{ id: "server", kind: "mcp", display_name: "Server", digest: "library-wrapper", comparison_digest: "same-content", path: "/hub/mcp/server", compatible_targets: ["codex"] }];
        if (command === "initial_scan") return [{ id: "found", kind: "mcp", source: "codex", source_key: "server:server", path: "/fixture/.codex/config.toml", digest: "tool-wrapper", comparison_digest: "same-content", selected: false, importable: true }];
        if (command === "debug_event") return;
        throw new Error(`Unexpected IPC ${command}`);
      } } });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "我的能力库", exact: true }).click();
    await page.getByRole("button", { name: "扫描并导入", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("tab", { name: /MCP/ }).click();
    await expect(dialog.getByText("库中已有相同内容")).toBeVisible();
    await expect(dialog.getByRole("checkbox")).toBeDisabled();
    await expect(dialog.getByRole("button", { name: "导入所选", exact: true })).toBeDisabled();
  });
}
