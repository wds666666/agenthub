import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import App from "./App";
import { api, type HostResource } from "./lib/api";

describe("AgentHub shell", () => {
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
    expect(screen.getByText(/后续修改会立即投影/)).toBeInTheDocument();
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
  expect(await screen.findByRole("heading", { name: "建立 AgentHub 库" })).toBeInTheDocument();
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
