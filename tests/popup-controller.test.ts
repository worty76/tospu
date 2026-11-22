import type { AnimationName } from "../src/domain/types";
import type { Backend, Snapshot } from "../src/ipc/contract";
import { PopupController } from "../src/popup/popup-controller";
import type { SpriteRenderer } from "../src/sprites/renderer";

const manifest = {
  schemaVersion: 1,
  id: "mochi",
  name: "Mochi",
  renderer: "spritesheet",
  frame: { width: 10, height: 10 },
  animations: {
    idle: { sheet: "idle.png", frames: 2, fps: 10, loop: true },
    enter: { sheet: "wave.png", frames: 2, fps: 10 },
    cheer: { sheet: "cheer.png", frames: 2, fps: 10 },
    exit: { sheet: "sleep.png", frames: 2, fps: 10 },
  },
  moods: { goal: "cheer" },
};

function snapshot(overrides: Partial<Snapshot["settings"]> = {}): Snapshot {
  return {
    settings: { activePack: "mochi", displaySeconds: 2, quietMode: false, ...overrides },
    packs: [{ id: "mochi", dir: "/packs/mochi", manifestJson: JSON.stringify(manifest) }],
    contentJson: JSON.stringify({ goals: ["Ship Tospu"], proud: ["Proud of you"] }),
    errors: [],
  };
}

function setup(snap = snapshot()) {
  const plays: AnimationName[] = [];
  const renderer: SpriteRenderer = {
    mount: () => {},
    // Every animation "takes" 200ms.
    play: (name) => {
      plays.push(name);
      return new Promise((r) => setTimeout(r, 200));
    },
    stop: () => {},
    destroy: () => {},
  };
  const backend: Backend = {
    getSnapshot: vi.fn(async () => snap),
    setHitRect: vi.fn(async () => {}),
    popupFinished: vi.fn(async () => {}),
    onShow: () => () => {},
    assetUrl: (dir, file) => `${dir}/${file}`,
  };
  const stage = document.createElement("div");
  document.body.replaceChildren(stage);
  const controller = new PopupController({ stage, backend, createRenderer: () => renderer, reducedMotion: false });
  const bubbleText = () => stage.querySelector(".bubble-text")?.textContent ?? "";
  const bubble = () => stage.querySelector<HTMLElement>(".bubble")!;
  return { controller, plays, backend, stage, bubbleText, bubble };
}

describe("PopupController", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("plays enter, the kind's mood, idles, then exits and tells the backend", async () => {
    const { controller, plays, backend, bubbleText } = setup();
    const done = controller.show("goal", false);
    await vi.advanceTimersByTimeAsync(5000);
    await done;

    expect(plays[0]).toBe("enter");
    expect(plays[1]).toBe("cheer");
    expect(plays.slice(2, -1).every((p) => p === "idle")).toBe(true);
    expect(plays.at(-1)).toBe("exit");
    expect(bubbleText()).toBe("Big goal: Ship Tospu");
    expect(backend.popupFinished).toHaveBeenCalledOnce();
    expect(backend.setHitRect).toHaveBeenLastCalledWith(null);
  });

  it("in quiet mode shows only 'Keep going.' until clicked", async () => {
    const { controller, bubbleText, bubble } = setup(snapshot({ quietMode: true }));
    void controller.show("proud", true);
    await vi.advanceTimersByTimeAsync(1500); // enter + type-in
    expect(bubbleText()).toBe("Keep going.");

    bubble().click();
    await vi.advanceTimersByTimeAsync(1000);
    expect(bubbleText()).toBe("Proud of you");
  });

  it("clicking a revealed bubble dismisses early", async () => {
    const { controller, plays, backend, bubble } = setup(snapshot({ displaySeconds: 60 }));
    const done = controller.show("proud", false);
    await vi.advanceTimersByTimeAsync(1000);
    bubble().click();
    await vi.advanceTimersByTimeAsync(1000);
    await done;
    expect(plays.at(-1)).toBe("exit");
    expect(backend.popupFinished).toHaveBeenCalledOnce();
  });

  it("hovering pauses the countdown", async () => {
    const { controller, backend, stage } = setup(snapshot({ displaySeconds: 2 }));
    void controller.show("proud", false);
    await vi.advanceTimersByTimeAsync(500);
    stage.dispatchEvent(new MouseEvent("mouseenter"));
    await vi.advanceTimersByTimeAsync(10_000);
    expect(backend.popupFinished).not.toHaveBeenCalled();
    stage.dispatchEvent(new MouseEvent("mouseleave"));
    await vi.advanceTimersByTimeAsync(5000);
    expect(backend.popupFinished).toHaveBeenCalledOnce();
  });

  it("ignores a second show while one is on screen", async () => {
    const { controller, backend } = setup();
    void controller.show("goal", false);
    await controller.show("quote", false);
    expect(backend.getSnapshot).toHaveBeenCalledTimes(1);
  });

  it("falls back to the first valid pack when the active one is broken", async () => {
    const snap = snapshot({ activePack: "broken" });
    snap.packs.push({ id: "broken", dir: "/packs/broken", manifestJson: "{ not json" });
    const { controller, plays } = setup(snap);
    void controller.show("goal", false);
    await vi.advanceTimersByTimeAsync(300);
    expect(plays[0]).toBe("enter");
  });
});
