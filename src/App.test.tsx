import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import App from "./App";
import { api, type HostResource } from "./lib/api";

describe("AgentHub shell", () => {
  it("renders the canonical switchboard and all four domains", async () => {
    render(<App/>);
    expect(await screen.findByRole("heading", { name: "能力交换台" })).toBeInTheDocument();
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
