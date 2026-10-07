import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import App from "./App";
import { api, type HostResource } from "./lib/api";
import { setLocale, t } from "./lib/i18n";

afterEach(() => { vi.restoreAllMocks(); setLocale("zh-CN"); });

describe("AgentHub shell", () => {
  it("restores without scanning, preserves a failed form and blocks duplicate requests", async () => {
    const dashboard = { initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] };
    vi.spyOn(api, "dashboard").mockImplementation(async () => dashboard);
    vi.spyOn(api, "inventory").mockResolvedValue([]);
    const scan = vi.spyOn(api, "scan");
    let resolve: (value: { imported: number; branch: string; recovery_path: string }) => void = () => undefined;
    const restore = vi.spyOn(api, "restoreLibrary").mockRejectedValueOnce(new Error("Authentication failed")).mockImplementationOnce(() => new Promise((done) => { resolve = done; }));
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /^恢复已有 AgentHub 库$/ }));
    fireEvent.change(screen.getByLabelText("远程仓库地址"), { target: { value: "https://github.com/tester/library.git" } });
    fireEvent.change(screen.getByLabelText("登录用户名"), { target: { value: "tester" } });
    const token = screen.getByLabelText(/^访问令牌$/);
    fireEvent.change(token, { target: { value: "fixture-only" } });
    expect(token).toHaveAttribute("type", "password");
    const submit = screen.getByRole("button", { name: /^认证并恢复能力库$/ });
    fireEvent.click(submit);
    expect(await screen.findByRole("alert")).toHaveTextContent("Authentication failed");
    expect(token).toHaveValue("fixture-only");
    fireEvent.click(submit);
    fireEvent.click(submit);
    expect(submit).toBeDisabled();
    expect(screen.getByRole("button", { name: /^返回选择$/ })).toBeDisabled();
    expect(restore).toHaveBeenCalledTimes(2);
    expect(scan).not.toHaveBeenCalled();
    dashboard.initialized = true;
    await act(async () => { resolve({ imported: 2, branch: "agenthub", recovery_path: "/test/backup" }); });
    expect(await screen.findByRole("heading", { name: "我的能力库" })).toBeInTheDocument();
    expect(restore).toHaveBeenCalledWith("https://github.com/tester/library.git", "", "tester", "fixture-only");
  });

  it("can initialize an empty library without scanning local tools", async () => {
    vi.spyOn(api, "dashboard").mockResolvedValue({ initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
    const scan = vi.spyOn(api, "scan");
    const finish = vi.spyOn(api, "finishInit").mockResolvedValue([]);
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /^从空库开始$/ }));
    await waitFor(() => expect(finish).toHaveBeenCalledWith([]));
    expect(scan).not.toHaveBeenCalled();
  });

  it("localizes first-run choices and restoration in both supported languages", () => {
    try {
      for (const locale of ["zh-CN", "en"] as const) {
        setLocale(locale);
        for (const key of ["welcome", "chooseHint", "restoreTitle", "restoreAction", "branchHint", "restoreScope", "restored"]) expect(t(`init.${key}`)).not.toBe(`init.${key}`);
      }
    } finally { setLocale("zh-CN"); }
  });
  it("renders the canonical switchboard and all four domains", async () => {
    render(<App/>);
    expect(await screen.findByRole("heading", { name: "能力管理中心" })).toBeInTheDocument();
    for (const label of ["Skills", "MCP", "Plugins", "Rules"]) expect(screen.getAllByText(label).length).toBeGreaterThan(0);
  });

  it("presents automatic sync as a persisted target profile", async () => {
    render(<App/>);
    fireEvent.click(await screen.findByRole("button", { name: /同步到工具/ }));
    expect(await screen.findByRole("heading", { name: "同步到工具" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /开启自动同步/ })).toBeDisabled();
    expect(screen.getByText(/更新工具从已保存版本读取/)).toBeInTheDocument();
  });

  it("selects only unimported deletable host resources and requires acknowledgement", async () => {
    const base = { target: "cursor", digest: "d", deletable: true } as const;
    const items: HostResource[] = [
      { ...base, id: "a", kind: "skill", display_name: "stray-skill", path: "/h/.cursor/skills/stray", relation: "host_only" },
      { ...base, id: "b", kind: "skill", display_name: "kept-skill", path: "/h/.cursor/skills/kept", relation: "canonical_match", canonical_id: "kept-skill" },
      { ...base, id: "c", kind: "plugin", display_name: "vendor", path: "/h/.cursor/plugins/x", relation: "constraint", deletable: false, constraint: "claude_plugins_cli_managed" },
    ];
    vi.spyOn(api, "hostInventory").mockResolvedValue(items);
    const cleanup = vi.spyOn(api, "cleanupHostResources").mockResolvedValue({ id: "x", deleted: ["a"], backup_path: "/b" });
    render(<App/>);
    fireEvent.click(await screen.findByRole("button", { name: /工具资源/ }));
    fireEvent.click(await screen.findByRole("button", { name: /一键选中未纳入 AgentHub 的资源 \(1\)/ }));
    fireEvent.click(screen.getByRole("button", { name: /删除所选工具资源/ }));
    const confirm = await screen.findByRole("button", { name: /备份并删除/ });
    expect(confirm).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: /确认从该工具中删除/ }));
    expect(confirm).toBeEnabled();
    fireEvent.click(confirm);
    await waitFor(() => expect(cleanup).toHaveBeenCalledWith("cursor", ["a"]));
  });
});

it("quick cleanup includes library matches but excludes protected stores", async () => {
  const items: HostResource[] = [
    { id: "local", target: "cursor", kind: "skill", display_name: "Local", path: "/h/local", digest: "a", relation: "host_only", deletable: true },
    { id: "match", target: "cursor", kind: "skill", display_name: "Match", path: "/h/match", digest: "b", relation: "canonical_match", deletable: true },
    { id: "protected", target: "cursor", kind: "plugin", display_name: "Protected", path: "/h/vendor", digest: "c", relation: "constraint", deletable: false },
  ];
  vi.spyOn(api, "hostInventory").mockResolvedValue(items);
  const cleanup = vi.spyOn(api, "cleanupHostResources").mockResolvedValue({ id: "cleanup", deleted: ["local", "match"], backup_path: "/backup" });
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /工具资源/ }));
  fireEvent.click(await screen.findByRole("button", { name: /快速清理全部可删除资源 \(2\)/ }));
  const confirm = await screen.findByRole("button", { name: /备份并删除/ });
  expect(confirm).toBeDisabled();
  expect(screen.getByText(/删除会产生差异/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("checkbox", { name: /确认从该工具中删除/ }));
  fireEvent.click(confirm);
  await waitFor(() => expect(cleanup).toHaveBeenCalledWith("cursor", ["local", "match"]));
});

it("requires typed reset confirmation and returns to import initialization", async () => {
  vi.spyOn(api, "policySettings").mockResolvedValue({ strict_authoritative: false, sync_after_reverse_import: false });
  const dashboard = vi.spyOn(api, "dashboard");
  const reset = vi.spyOn(api, "resetAgenthub").mockResolvedValue("/recovery");
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /设置与诊断/ }));
  fireEvent.click(await screen.findByRole("button", { name: "重置并重新导入" }));
  const dialog = screen.getByRole("dialog");
  const confirm = dialog.querySelector("button.button--danger") ?? Array.from(dialog.querySelectorAll("button")).find((button) => button.textContent?.includes("重置并重新导入"))!;
  expect(confirm).toBeDisabled();
  fireEvent.change(screen.getByRole("textbox", { name: /请输入 AGENTHUB/ }), { target: { value: "AGENTHUB" } });
  expect(confirm).toBeEnabled();
  dashboard.mockResolvedValue({ initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
  fireEvent.click(confirm);
  await waitFor(() => expect(reset).toHaveBeenCalledWith("AGENTHUB"));
  expect(await screen.findByRole("heading", { name: "欢迎使用 AgentHub" })).toBeInTheDocument();
});

it("keeps two Cursor and one Codex selections across source filters", async () => {
  const items = [
    { id: "cursor-1", source: "cursor", path: "/h/.cursor/skills/first", digest: "one" },
    { id: "cursor-2", source: "cursor", path: "/h/.cursor/skills/second", digest: "two" },
    { id: "codex-1", source: "codex", path: "/h/.codex/skills/third", digest: "three" },
  ].map((item) => ({ ...item, kind: "skill" as const, selected: false, importable: true }));
  vi.spyOn(api, "dashboard").mockResolvedValue({ initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
  vi.spyOn(api, "scan").mockResolvedValue(items);
  const finish = vi.spyOn(api, "finishInit").mockResolvedValue(["first", "second", "third"]);
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /开始全局扫描/ }));
  fireEvent.click(await screen.findByRole("button", { name: /Cursor 0 \/ 2/ }));
  fireEvent.click(screen.getByRole("button", { name: /选择当前来源全部/ }));
  fireEvent.click(screen.getByRole("button", { name: /Codex 0 \/ 1/ }));
  expect(screen.queryByText("first")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /选择当前来源全部/ }));
  expect(screen.getByRole("status")).toHaveTextContent("共选择 3 · 去重后预计导入 3");
  fireEvent.click(screen.getByRole("button", { name: /Cursor 2 \/ 2/ }));
  expect(screen.getAllByRole("checkbox").every((input) => (input as HTMLInputElement).checked)).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: /导入所选并完成/ }));
  await waitFor(() => expect(finish).toHaveBeenCalledWith(["cursor-1", "cursor-2", "codex-1"]));
});

it("shows all supported import sources even when only Claude has discoveries", async () => {
  vi.spyOn(api, "dashboard").mockResolvedValue({ initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
  vi.spyOn(api, "scan").mockResolvedValue([{ id: "claude-only", source: "claude", path: "/h/.claude/skills/review", digest: "one", kind: "skill", selected: false, importable: true }]);
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /开始全局扫描/ }));
  fireEvent.click(await screen.findByRole("button", { name: /Codex 0 \/ 0/ }));
  expect(screen.getByRole("button", { name: /Cursor 0 \/ 0/ })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /共享 Agents 0 \/ 0/ })).toBeInTheDocument();
  expect(screen.getByText(/此来源未发现资源/)).toBeInTheDocument();
});

it("uses the latest selection for rapid source and category toggles", async () => {
  vi.spyOn(api, "dashboard").mockResolvedValue({ initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
  vi.spyOn(api, "scan").mockResolvedValue([{ id: "one", source: "agents", path: "/h/.agents/skills/parent", digest: "one", kind: "skill", selected: false, importable: true }]);
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /开始全局扫描/ }));
  const source = await screen.findByRole("button", { name: "选择当前来源全部" });
  act(() => { source.click(); source.click(); });
  expect(screen.getByRole("status")).toHaveTextContent("共选择 0");
  const category = screen.getByRole("button", { name: "选择本类" });
  act(() => { category.click(); category.click(); });
  expect(screen.getByRole("status")).toHaveTextContent("共选择 0");
  expect(screen.getByRole("heading", { name: "建立 AgentHub 库" })).toBeInTheDocument();
});

it("blocks repeated import, preserves selection on failure and allows retry", async () => {
  vi.spyOn(api, "dashboard").mockResolvedValue({ initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
  vi.spyOn(api, "scan").mockResolvedValue([{ id: "parent", source: "agents", path: "/h/.agents/skills/parent", digest: "one", kind: "skill", selected: false, importable: true }]);
  let rejectImport!: (error: Error) => void;
  const finish = vi.spyOn(api, "finishInit").mockImplementationOnce(() => new Promise((_, reject) => { rejectImport = reject; })).mockResolvedValueOnce(["parent"]);
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /开始全局扫描/ }));
  fireEvent.click(await screen.findByRole("checkbox"));
  const submit = screen.getByRole("button", { name: /导入所选并完成/ });
  act(() => { submit.click(); submit.click(); });
  expect(finish).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("checkbox")).toBeDisabled();
  expect(screen.getAllByRole("tab").every((tab) => (tab as HTMLButtonElement).disabled)).toBe(true);
  await act(async () => rejectImport(new Error("Import failed")));
  expect(screen.getByRole("alert")).toHaveTextContent("Import failed");
  expect(screen.getByRole("checkbox")).toBeChecked();
  fireEvent.click(screen.getByRole("button", { name: /导入所选并完成/ }));
  await waitFor(() => expect(finish).toHaveBeenCalledTimes(2));
  expect(finish).toHaveBeenLastCalledWith(["parent"]);
});

it("explains skipped runtime directories and excludes unsafe skills from select all", async () => {
  vi.spyOn(api, "dashboard").mockResolvedValue({ initialized: false, inventory: {}, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
  vi.spyOn(api, "scan").mockResolvedValue([
    { id: "pdf", source: "agents", path: "/h/.agents/skills/pdf-read", digest: "portable", kind: "skill", selected: false, importable: true, warning: "skill_runtime_excluded", warning_detail: ".venv" },
    { id: "unsafe", source: "agents", path: "/h/.agents/skills/unsafe", digest: "invalid", kind: "skill", selected: false, importable: false, warning: "skill_symlink", warning_detail: "scripts/external.py" },
  ]);
  const finish = vi.spyOn(api, "finishInit").mockResolvedValue(["pdf-read"]);
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /开始全局扫描/ }));
  expect(await screen.findByText("已跳过本机环境和缓存，保留技能文件与依赖清单")).toBeInTheDocument();
  expect(screen.getByText(".venv")).toBeInTheDocument();
  expect(screen.getByText("scripts/external.py")).toBeInTheDocument();
  const checkboxes = screen.getAllByRole("checkbox");
  expect(checkboxes[0]).toBeEnabled();
  expect(checkboxes[1]).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: /^全选$/ }));
  expect(checkboxes[0]).toBeChecked();
  expect(checkboxes[1]).not.toBeChecked();
  fireEvent.click(screen.getByRole("button", { name: /导入所选并完成/ }));
  await waitFor(() => expect(finish).toHaveBeenCalledWith(["pdf"]));
});

it("preserves cross-kind selection through search and confirms the exact batch", async () => {
  vi.spyOn(api, "dashboard").mockResolvedValue({ initialized: true, inventory: { skill: 1, rule: 1 }, enabled_targets: [], auto_sync_targets: [], dirty: false, recent_transactions: [] });
  const capabilities = [
    { id: "same", kind: "skill" as const, display_name: "Review skill", digest: "skill", path: "/hub/skills/same", compatible_targets: [] },
    { id: "same", kind: "rule" as const, display_name: "Safety rule", digest: "rule", path: "/hub/rules/same", compatible_targets: [] },
  ];
  const inventory = vi.spyOn(api, "inventory").mockResolvedValue(capabilities);
  const remove = vi.spyOn(api, "deleteCapabilities").mockResolvedValue({ deleted: capabilities.map(({ kind, id }) => ({ kind, id })), backup_path: "/hub/backups/library-delete-test", auto_sync: [] });
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /我的能力库/ }));
  fireEvent.click(await screen.findByRole("button", { name: "选择本类 Skills" }));
  fireEvent.change(screen.getByRole("textbox", { name: "搜索能力" }), { target: { value: "Safety" } });
  fireEvent.click(screen.getByRole("button", { name: "选择当前结果" }));
  expect(screen.getByRole("status")).toHaveTextContent("已选择 2");
  fireEvent.click(screen.getByRole("button", { name: "删除所选能力" }));
  expect(remove).not.toHaveBeenCalled();
  const dialog = screen.getByRole("dialog");
  expect(dialog).toHaveTextContent("Review skill");
  expect(dialog).toHaveTextContent("Safety rule");
  inventory.mockResolvedValue([]);
  fireEvent.click(screen.getByRole("button", { name: "删除所选能力 (2)" }));
  await waitFor(() => expect(remove).toHaveBeenCalledWith([{ kind: "skill", id: "same" }, { kind: "rule", id: "same" }]));
  expect(await screen.findByText("/hub/backups/library-delete-test")).toBeInTheDocument();
});

it("separates configured repositories from verified access and keeps tokens out of browser storage", async () => {
  vi.spyOn(api, "remoteSettings").mockResolvedValue({ url: "https://git.example/team/skills.git", branch: "agenthub", state: "unverified", credential_saved: false });
  const login = vi.spyOn(api, "loginRemote").mockRejectedValueOnce(new Error("Authentication failed")).mockResolvedValue({ url: "https://git.example/team/skills.git", branch: "agenthub", state: "read_verified", credential_saved: true });
  const sync = vi.spyOn(api, "syncRemote").mockResolvedValue({ auto_sync: [] });
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /^版本记录$/ }));
  expect(await screen.findByText("仓库已配置，尚未验证")).toBeInTheDocument();
  expect(screen.queryByText("上次验证可读取")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /^仓库登录$/ }));
  const token = screen.getByLabelText(/^访问令牌$/);
  expect(token).toHaveAttribute("type", "password");
  fireEvent.change(screen.getByLabelText("登录用户名"), { target: { value: "tester" } });
  fireEvent.change(token, { target: { value: "fixture-token" } });
  fireEvent.click(screen.getByRole("button", { name: "显示访问令牌" }));
  expect(token).toHaveAttribute("type", "text");
  fireEvent.click(screen.getByRole("button", { name: "验证并保存登录" }));
  expect(await screen.findByText("Error: Authentication failed")).toBeInTheDocument();
  expect(token).toHaveValue("fixture-token");
  fireEvent.click(screen.getByRole("button", { name: "验证并保存登录" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  expect(login).toHaveBeenLastCalledWith("https://git.example/team/skills.git", "agenthub", "tester", "fixture-token");
  expect(sync).not.toHaveBeenCalled();
  expect(screen.getByText("上次验证可读取")).toBeInTheDocument();
  expect(JSON.stringify(localStorage)).not.toContain("fixture-token");
  fireEvent.click(screen.getByRole("button", { name: /^仓库登录$/ }));
  expect(screen.getByLabelText(/^访问令牌$/)).toHaveValue("");
});

it("automatically marks changed Skills and reviews copies without a manual scan", async () => {
  const item = { id: "changed", kind: "skill", source: "agents", path: "/fixture/.agents/skills/example", digest: "new", selected: false, importable: true } as const;
  const check = vi.spyOn(api, "checkSkillChanges").mockResolvedValue({ changes: [{ canonical_id: "example", item }], errors: [] });
  vi.spyOn(api, "inventory").mockResolvedValue([{ id: "example", kind: "skill", display_name: "Example", path: "/fixture/hub/skills/example", digest: "old", compatible_targets: ["agents"] }]);
  const scan = vi.spyOn(api, "scan");
  const imported = vi.spyOn(api, "importScanned").mockResolvedValue({ imported: ["example-2"], skipped_duplicates: 0, auto_sync: [] });
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: /我的能力库 · 项工具侧 Skill 内容有变化 1/ }));
  fireEvent.click(await screen.findByRole("button", { name: "查看 Skill 变化 Example" }));
  expect(await screen.findByRole("dialog")).toHaveTextContent("原项会保留");
  expect(scan).not.toHaveBeenCalled();
  const submit = screen.getByRole("button", { name: "导入所选" });
  expect(submit).toBeDisabled();
  fireEvent.click(screen.getByRole("checkbox", { name: /example.*agents/ }));
  expect(submit).toBeEnabled();
  check.mockResolvedValue({ changes: [], errors: [] });
  fireEvent.click(submit);
  await waitFor(() => expect(imported).toHaveBeenCalledWith(["changed"]));
  await waitFor(() => expect(screen.queryByRole("button", { name: "查看 Skill 变化 Example" })).not.toBeInTheDocument());
});

it("throttles navigation and focus checks and retries a partial failure explicitly", async () => {
  let now = 1000;
  vi.spyOn(Date, "now").mockImplementation(() => now);
  const check = vi.spyOn(api, "checkSkillChanges").mockResolvedValue({ changes: [], errors: ["/unreadable/skill"] });
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: "我的能力库" }));
  expect(await screen.findByText("部分 Skill 未能完成检查，请重试。")).toBeInTheDocument();
  fireEvent.focus(window);
  expect(check).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "重试" }));
  await waitFor(() => expect(check).toHaveBeenCalledWith(true));
  check.mockResolvedValue({ changes: [], errors: [] });
  now += 120_001;
  fireEvent.focus(window);
  await waitFor(() => expect(check).toHaveBeenCalledTimes(3));
  expect(screen.queryByText("部分 Skill 未能完成检查，请重试。")).not.toBeInTheDocument();
});
