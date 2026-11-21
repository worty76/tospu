import { animationForKind, parseManifest, resolveAnimation } from "../src/sprites/manifest";

const minimal = {
  schemaVersion: 1,
  id: "blob",
  name: "Blob",
  renderer: "spritesheet",
  frame: { width: 64, height: 64 },
  animations: { idle: { sheet: "idle.png", frames: 4, fps: 8, loop: true } },
};

const parsed = (json: unknown) => {
  const r = parseManifest(json);
  if (!r.ok) throw new Error(r.error);
  return r.manifest;
};

describe("parseManifest", () => {
  it("accepts a minimal pack with only idle", () => {
    const m = parsed(minimal);
    expect(m.frame.displayHeight).toBe(120);
    expect(m.pixelated).toBe(false);
  });

  it("accepts the bundled Mochi manifest", async () => {
    const json = (await import("../sprites/mochi/manifest.json")).default;
    expect(parseManifest(json).ok).toBe(true);
  });

  it("rejects a pack without idle and names the missing field", () => {
    const r = parseManifest({ ...minimal, animations: { cheer: minimal.animations.idle } });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toContain("animations.idle");
  });

  it("rejects ids that are not folder-safe", () => {
    expect(parseManifest({ ...minimal, id: "../evil" }).ok).toBe(false);
  });
});

describe("resolveAnimation", () => {
  it("falls back to idle when an animation is missing", () => {
    const m = parsed(minimal);
    expect(resolveAnimation(m, "cheer")).toEqual(m.animations.idle);
    expect(resolveAnimation(m, "exit")).toEqual(m.animations.idle);
  });
});

describe("animationForKind", () => {
  it("uses the mood map, defaulting to idle", () => {
    const m = parsed({ ...minimal, moods: { goal: "cheer" } });
    expect(animationForKind(m, "goal")).toBe("cheer");
    expect(animationForKind(m, "quote")).toBe("idle");
  });
});
