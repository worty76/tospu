import { LocalContentSource, FALLBACK_LINE } from "../src/content/content-source";
import { DEFAULT_CONTENT, parseContent, type Content } from "../src/content/schema";

const empty: Content = { goals: [], quotes: [], gratitude: [], proud: [] };

/** Deterministic random: cycles through the given values. */
const seq = (...xs: number[]) => {
  let i = 0;
  return () => xs[i++ % xs.length]!;
};

describe("parseContent", () => {
  it("accepts quotes as plain strings or {text, author}", () => {
    const r = parseContent({ quotes: ["a", { text: "b", author: "Seneca" }] });
    expect(r.error).toBeUndefined();
    expect(r.content.quotes).toEqual([{ text: "a" }, { text: "b", author: "Seneca" }]);
    expect(r.content.goals).toEqual([]);
  });

  it("returns defaults plus an error for invalid content", () => {
    const r = parseContent({ goals: "not a list" });
    expect(r.content).toEqual(DEFAULT_CONTENT);
    expect(r.error).toContain("goals");
  });
});

describe("LocalContentSource", () => {
  it("formats goals with their title", () => {
    const src = new LocalContentSource({ ...empty, goals: [{ title: "Ship Yub" }] });
    expect(src.next("goal")).toEqual({ kind: "goal", text: "Big goal: Ship Yub" });
  });

  it("does not repeat the previous message when there are alternatives", () => {
    const content = { ...empty, quotes: [{ text: "a" }, { text: "b" }, { text: "c" }] };
    const src = new LocalContentSource(content, { historySize: 2, random: seq(0, 0, 0, 0) });
    const seen = [src.next("quote").text, src.next("quote").text, src.next("quote").text];
    expect(new Set(seen).size).toBe(3);
  });

  it("falls back to another kind when the asked kind is empty", () => {
    const src = new LocalContentSource({ ...empty, gratitude: ["my family"] });
    expect(src.next("proud")).toEqual({ kind: "gratitude", text: "Grateful for: my family" });
  });

  it("never throws when everything is empty", () => {
    const src = new LocalContentSource(empty);
    expect(src.next("goal")).toEqual({ kind: "goal", text: FALLBACK_LINE });
  });
});
