import { expect, test } from "@playwright/test";

test("startup and navigation never apply saved automatic profiles to cleared tools", async ({ page }) => {
  await page.addInitScript(() => {
    const mutations: string[] = [];
    Object.defineProperty(window, "fixtureHostMutations", { value: mutations });
    const selection = { mode: "preserve", skills_managed: true, skills: Array.from({ length: 24 }, (_, index) => `existing-${index}`), plugins: [], mcp: [], rule_ids: [] };
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      if (command === "dashboard") return { initialized: true, inventory: { skill: 24 }, enabled_targets: ["agents"], auto_sync_targets: ["agents"], dirty: false, recent_transactions: [] };
      if (command === "runtime_diagnostics") return { log_dir: "/fixture/logs", canonical_root: "/fixture/hub", git_available: true, platform: "windows" };
      if (command === "inventory") return selection.skills.map((id) => ({ id, kind: "skill", display_name: id, path: `/fixture/hub/skills/${id}`, digest: id, compatible_targets: ["agents"] }));
      if (command === "auto_sync_errors") return [];
      if (command === "check_skill_changes") return { changes: [], errors: [] };
      if (command === "host_inventory") return [];
      if (command === "auto_sync_profiles") return [{ target: "agents", enabled: true, needs_review: false, selection }];
      if (command === "policy_settings") return { sync_mode: "preserve", strict_authoritative: false, sync_after_reverse_import: false };
      if (command === "debug_event") return;
      mutations.push(command);
      throw new Error(`Startup invoked unexpected operation: ${command}`);
    } } });
  });
  await page.goto("/");
  for (const name of ["我的能力库", "工具资源", "同步到工具", "总览"]) {
    await page.getByRole("button", { name, exact: true }).click();
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  }
  await expect.poll(() => page.evaluate(() => (window as unknown as { fixtureHostMutations: string[] }).fixtureHostMutations)).toEqual([]);
});
