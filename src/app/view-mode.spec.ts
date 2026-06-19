import { describe, expect, it } from "vitest";
import { expandViewMode, initialViewMode } from "./view-mode";

describe("view mode", () => {
  it("starts in compact mode and can expand to full mode", () => {
    expect(initialViewMode()).toBe("compact");
    expect(expandViewMode("compact")).toBe("full");
    expect(expandViewMode("full")).toBe("full");
  });
});

