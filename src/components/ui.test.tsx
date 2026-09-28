import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Button, Dialog } from "./ui";

describe("Dialog interaction boundary", () => {
  afterEach(() => {
    document.querySelectorAll("#root").forEach((element) => element.remove());
    document.body.style.overflow = "";
  });

  it("portals outside the inert application root and keeps actions clickable", async () => {
    vi.spyOn(console, "debug").mockImplementation(() => undefined);
    const appRoot = document.createElement("div");
    appRoot.id = "root";
    document.body.append(appRoot);
    const confirm = vi.fn();

    const view = render(
      <Dialog
        open
        title="Confirm sync"
        onClose={() => undefined}
        actions={<Button onClick={confirm}>Apply</Button>}
      >
        <p>Review the plan.</p>
      </Dialog>,
      { container: appRoot },
    );

    const dialog = screen.getByRole("dialog", { name: "Confirm sync" });
    await waitFor(() => expect(appRoot.inert).toBe(true));
    expect(appRoot.contains(dialog)).toBe(false);
    expect(document.body.contains(dialog)).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(confirm).toHaveBeenCalledTimes(1);

    view.unmount();
    expect(appRoot.inert).toBe(false);
  });
});
