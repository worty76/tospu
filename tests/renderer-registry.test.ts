import { parseManifest, type SpriteManifest } from "../src/sprites/manifest";
import { UnknownRendererError, createRenderer, registerRenderer } from "../src/sprites/renderer";
import { SpriteSheetRenderer } from "../src/sprites/spritesheet-renderer";

const manifest = (renderer: string): SpriteManifest => {
  const r = parseManifest({
    schemaVersion: 1,
    id: "blob",
    name: "Blob",
    renderer,
    frame: { width: 10, height: 10 },
    animations: {
      idle: { sheet: "idle.png", frames: 2, fps: 10, loop: true },
      exit: { sheet: "exit.png", frames: 3, fps: 10, loop: false },
    },
  });
  if (!r.ok) throw new Error(r.error);
  return r.manifest;
};

const pack = (renderer: string) => ({ manifest: manifest(renderer), assetUrl: (f: string) => `asset://${f}` });

describe("renderer registry", () => {
  it("creates the built-in spritesheet renderer", () => {
    expect(createRenderer(pack("spritesheet"))).toBeInstanceOf(SpriteSheetRenderer);
  });

  it("throws UnknownRendererError for an unregistered kind", () => {
    expect(() => createRenderer(pack("lottie"))).toThrow(UnknownRendererError);
  });

  it("lets new renderers register without touching existing code", () => {
    const fake = { mount() {}, play: async () => {}, stop() {}, destroy() {} };
    registerRenderer("test-kind", () => fake);
    expect(createRenderer(pack("test-kind"))).toBe(fake);
  });
});

describe("SpriteSheetRenderer", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("steps frames and resolves a one-shot animation after its last frame", async () => {
    const r = new SpriteSheetRenderer(pack("spritesheet"));
    const host = document.createElement("div");
    r.mount(host);
    const el = host.firstElementChild as HTMLElement;
    let done = false;
    void r.play("exit").then(() => (done = true));

    expect(el.style.backgroundImage).toContain("asset://exit.png");
    await vi.advanceTimersByTimeAsync(100);
    // frame 2 of 3; 10x10 frames shown at the default 120px height → 120px wide
    expect(el.style.backgroundPosition).toBe("-120px 0px");
    await vi.advanceTimersByTimeAsync(200);
    expect(done).toBe(true);
  });

  it("falls back to idle for a missing animation", () => {
    const r = new SpriteSheetRenderer(pack("spritesheet"));
    const host = document.createElement("div");
    r.mount(host);
    void r.play("cheer");
    expect((host.firstElementChild as HTMLElement).style.backgroundImage).toContain("idle.png");
  });
});
