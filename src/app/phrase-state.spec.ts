import { describe, expect, it } from "vitest";
import { addPhrase, deletePhrase, movePhrase, updatePhrase } from "./phrase-state";
import type { AppConfig } from "./types";

const baseConfig = (): AppConfig => ({
  hotkey: "Ctrl+Alt+Space",
  phrases: [
    { id: "go-on", label: "Go on", text: "go on", enabled: true },
    { id: "commit", label: "Commit", text: "commit", enabled: true },
  ],
});

describe("phrase state", () => {
  it("adds a phrase with a stable id and safe defaults", () => {
    const next = addPhrase(baseConfig(), "Run tests", "run tests");

    expect(next.phrases).toHaveLength(3);
    expect(next.phrases[2]).toMatchObject({
      id: "run-tests",
      label: "Run tests",
      text: "run tests",
      enabled: true,
    });
  });

  it("updates one phrase without mutating the original config", () => {
    const original = baseConfig();
    const next = updatePhrase(original, "commit", { text: "commit\nrun tests", enabled: false });

    expect(original.phrases[1].enabled).toBe(true);
    expect(next.phrases[1]).toMatchObject({ enabled: false, text: "commit\nrun tests" });
  });

  it("deletes a phrase by id", () => {
    const next = deletePhrase(baseConfig(), "go-on");

    expect(next.phrases.map((phrase) => phrase.id)).toEqual(["commit"]);
  });

  it("moves phrases up and down while clamping at list boundaries", () => {
    const movedDown = movePhrase(baseConfig(), "go-on", 1);
    const clamped = movePhrase(movedDown, "go-on", 1);
    const movedUp = movePhrase(clamped, "go-on", -1);

    expect(movedDown.phrases.map((phrase) => phrase.id)).toEqual(["commit", "go-on"]);
    expect(clamped.phrases.map((phrase) => phrase.id)).toEqual(["commit", "go-on"]);
    expect(movedUp.phrases.map((phrase) => phrase.id)).toEqual(["go-on", "commit"]);
  });
});


