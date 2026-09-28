import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import App from "./App";

describe("AgentHub shell", () => {
  it("renders the canonical switchboard and all four domains", async () => {
    render(<App/>);
    expect(await screen.findByRole("heading", { name: "能力交换台" })).toBeInTheDocument();
    for (const label of ["Skills", "MCP", "Plugins", "Rules"]) expect(screen.getAllByText(label).length).toBeGreaterThan(0);
  });
});
