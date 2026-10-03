import { expect, test, type Page } from "@playwright/test";

async function mockLibrary(page: Page, initialized: boolean) {
  // IPC is isolated: this workflow never scans or writes a developer's home.
  await page.addInitScript(({ initialized }) => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      if (command === "dashboard") return { initialized, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
      if (command === "runtime_diagnostics") return { log_dir: "/test/logs", canonical_root: "/test/hub", git_available: true, platform: "linux" };
      if (command === "inventory") return [];
      if (command === "initial_scan") return Array.from({ length: 96 }, (_, index) => ({ id: `item-${index}`, kind: "skill", source: index < 50 ? "agents" : "claude", path: `/test/.agents/skills/parent-${index}`, digest: `digest-${index}`, selected: false, importable: true }));
      if (command === "debug_event") return;
      throw new Error(`Unexpected IPC: ${command}`);
    } } });
  }, { initialized });
}

async function expectBoundedCheckbox(page: Page, index: number) {
  const row = page.locator(".scan-list label").nth(index);
  await row.scrollIntoViewIfNeeded();
  await row.locator("input").focus();
  const geometry = await row.evaluate((element) => {
    const row = element.getBoundingClientRect();
    const input = element.querySelector("input")!.getBoundingClientRect();
    const list = element.closest(".scan-list")!.getBoundingClientRect();
    return { contained: input.left >= row.left && input.right <= row.right && input.top >= row.top && input.bottom <= row.bottom, visible: input.bottom > list.top && input.top < list.bottom, pageOverflow: document.documentElement.scrollWidth > innerWidth };
  });
  expect(geometry).toEqual({ contained: true, visible: true, pageOverflow: false });
}

for (const viewport of [{ width: 1280, height: 800 }, { width: 1280, height: 600 }, { width: 760, height: 760 }, { width: 390, height: 844 }]) {
  test(`initial import stays visible after repeated clicks at ${viewport.width}x${viewport.height}`, async ({ page }, testInfo) => {
    await page.setViewportSize(viewport);
    await mockLibrary(page, false);
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto("/");
    await page.getByRole("button", { name: "开始全局扫描", exact: true }).click();
    await expect(page.locator(".scan-list label")).toHaveCount(96);
    for (let index = 0; index < 20; index++) await page.locator(".scan-list label").nth(index).click({ delay: 5 });
    await expectBoundedCheckbox(page, 95);
    await page.keyboard.press("Space");
    await expect(page.locator(".scan-list input").last()).toBeChecked();
    // Invisible absolute inputs previously added thousands of pixels of overflow.
    const phantomOverflow = await page.locator(".init-panel").evaluate((panel) => {
      const footer = panel.querySelector(".scan-footer")!.getBoundingClientRect();
      const bottom = footer.bottom - panel.getBoundingClientRect().top + panel.scrollTop + parseFloat(getComputedStyle(panel).paddingBottom);
      return panel.scrollHeight - bottom;
    });
    expect(phantomOverflow).toBeLessThanOrEqual(2);
    expect(await page.evaluate(() => document.documentElement.scrollHeight - innerHeight)).toBeLessThanOrEqual(1);
    await page.getByRole("button", { name: /Codex 0 \/ 0/ }).click();
    await expect(page.getByText("未发现此类能力", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: /全部来源/ }).click();
    await expect(page.locator(".scan-list input:checked")).toHaveCount(21);
    await page.getByRole("button", { name: "清空选择", exact: true }).click();
    await expect(page.locator(".scan-list input:checked")).toHaveCount(0);
    await page.screenshot({ path: testInfo.outputPath("import.png") });
    expect(errors).toEqual([]);
  });

  test(`later import dialog preserves row focus and selection at ${viewport.width}x${viewport.height}`, async ({ page }, testInfo) => {
    await page.setViewportSize(viewport);
    await mockLibrary(page, true);
    await page.goto("/");
    await page.getByRole("button", { name: "我的能力库", exact: true }).click();
    await page.getByRole("button", { name: "扫描并导入", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    for (let index = 0; index < 20; index++) await page.locator(".scan-list label").nth(index).click({ delay: 5 });
    await expectBoundedCheckbox(page, 95);
    await page.keyboard.press("Space");
    await expect(page.locator(".scan-list input:checked")).toHaveCount(21);
    await page.getByRole("button", { name: /Codex 0 \/ 0/ }).click();
    await page.getByRole("button", { name: /全部来源/ }).click();
    await expect(page.locator(".scan-list input:checked")).toHaveCount(21);
    await page.screenshot({ path: testInfo.outputPath("import-dialog.png") });
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "扫描并导入", exact: true })).toBeFocused();
  });
}
