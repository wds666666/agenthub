import { fireEvent, render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import App from "./App";

describe("AgentHub shell", () => {
  it("renders the canonical switchboard and all four domains", async () => {
    render(<App/>);
    expect(await screen.findByRole("heading", { name: "能力交换台" })).toBeInTheDocument();
    for (const label of ["Skills", "MCP", "Plugins", "Rules"]) expect(screen.getAllByText(label).length).toBeGreaterThan(0);
  });

  it("presents automatic sync as a persisted target profile", async () => {
    render(<App/>);
    fireEvent.click(await screen.findByRole("button", { name: /目标同步/ }));
    expect(await screen.findByRole("heading", { name: "目标同步" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /开启自动同步/ })).toBeDisabled();
    expect(screen.getByText(/后续修改会立即投影/)).toBeInTheDocument();
  });
});
