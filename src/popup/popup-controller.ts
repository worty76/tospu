import { LocalContentSource } from "../content/content-source";
import { parseContent } from "../content/schema";
import { QUIET_TEXT, type Message, type MessageKind } from "../domain/types";
import type { Backend, Snapshot } from "../ipc/contract";
import { animationForKind } from "../sprites/manifest";
import { choosePack, loadPacks } from "../sprites/pack-loader";
import { createRenderer as defaultCreateRenderer, type LoadedPack, type SpriteRenderer } from "../sprites/renderer";
import { Bubble } from "./bubble";

const TICK_MS = 100;

export interface PopupControllerDeps {
  stage: HTMLElement;
  backend: Backend;
  createRenderer?: (pack: LoadedPack) => SpriteRenderer;
  /** Skip movement (macOS "Reduce motion"): static sprite, instant text. */
  reducedMotion?: boolean;
}

/**
 * Runs one popup: enter → say → talk/idle (paused while hovered) → exit.
 * Knows nothing about which character or image format is on screen.
 */
export class PopupController {
  private readonly stage: HTMLElement;
  private readonly backend: Backend;
  private readonly makeRenderer: (pack: LoadedPack) => SpriteRenderer;
  private readonly reducedMotion: boolean;
  private readonly bubble: Bubble;
  private readonly spriteHost: HTMLDivElement;

  private busy = false;
  private hovered = false;
  private onClick: (() => void) | null = null;
  private content: { json: string | null; source: LocalContentSource } | null = null;
  private renderer: { key: string; renderer: SpriteRenderer } | null = null;

  constructor(deps: PopupControllerDeps) {
    this.stage = deps.stage;
    this.backend = deps.backend;
    this.makeRenderer = deps.createRenderer ?? defaultCreateRenderer;
    this.reducedMotion = deps.reducedMotion ?? false;

    this.bubble = new Bubble(this.reducedMotion);
    this.spriteHost = document.createElement("div");
    this.spriteHost.className = "sprite-host";
    this.stage.classList.add("stage");
    this.stage.append(this.bubble.el, this.spriteHost);

    this.stage.addEventListener("mouseenter", () => (this.hovered = true));
    this.stage.addEventListener("mouseleave", () => (this.hovered = false));
    this.bubble.el.addEventListener("click", () => this.onClick?.());
    this.spriteHost.addEventListener("click", () => this.onClick?.());
  }

  /** Shows one popup and resolves when it's gone. Ignored if one is showing. */
  async show(kind: MessageKind, quiet: boolean): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try {
      await this.run(kind, quiet);
    } catch (e) {
      console.error("[tospu] popup failed", e);
    } finally {
      this.onClick = null;
      this.bubble.hide();
      this.stage.classList.remove("visible");
      await this.backend.setHitRect(null);
      await this.backend.popupFinished();
      this.busy = false;
    }
  }

  private async run(kind: MessageKind, quiet: boolean): Promise<void> {
    const snap = await this.backend.getSnapshot();
    const pack = this.pickPack(snap);
    const message = this.contentSource(snap).next(kind);
    const renderer = pack ? this.rendererFor(pack) : null;
    const displayMs = Math.max(1, snap.settings.displaySeconds) * 1000;

    this.stage.classList.add("visible");
    await this.animate(renderer, "enter");

    let revealed = !quiet;
    const say = async () => {
      await (revealed ? this.bubble.say(message.text, message.author) : this.bubble.say(QUIET_TEXT));
      this.reportHitRect();
    };
    await say();

    let remaining = displayMs;
    let dismissed = false;
    this.onClick = () => {
      if (revealed) return void (dismissed = true);
      revealed = true;
      remaining = displayMs; // give time to read what was just revealed
      void say();
    };

    const talking = this.talk(renderer, pack, message);
    await new Promise<void>((resolve) => {
      const timer = setInterval(() => {
        if (!this.hovered) remaining -= TICK_MS;
        if (dismissed || remaining <= 0) {
          clearInterval(timer);
          resolve();
        }
      }, TICK_MS);
    });
    talking.stop();
    await talking.done;

    this.onClick = null;
    this.bubble.hide();
    await this.animate(renderer, "exit");
  }

  /** Mood animation once, then idle on repeat until stopped. */
  private talk(renderer: SpriteRenderer | null, pack: LoadedPack | undefined, message: Message) {
    let stopped = false;
    const run = async () => {
      if (!renderer || !pack) return;
      if (this.reducedMotion) return this.showStill(renderer);
      await renderer.play(animationForKind(pack.manifest, message.kind));
      while (!stopped) await renderer.play("idle");
    };
    const done = run();
    return {
      done,
      stop: () => {
        stopped = true;
        renderer?.stop();
      },
    };
  }

  private async animate(renderer: SpriteRenderer | null, name: "enter" | "exit") {
    if (!renderer) return;
    if (this.reducedMotion) return this.showStill(renderer);
    await renderer.play(name);
  }

  private showStill(renderer: SpriteRenderer) {
    void renderer.play("idle");
    renderer.stop(); // stays on the first idle frame
  }

  private pickPack(snap: Snapshot): LoadedPack | undefined {
    const { packs, errors } = loadPacks(snap.packs, this.backend.assetUrl);
    for (const e of [...snap.errors, ...errors]) console.warn("[tospu]", e);
    return choosePack(packs, snap.settings.activePack);
  }

  /** Re-parse content only when content.json changed, keeping repeat history. */
  private contentSource(snap: Snapshot): LocalContentSource {
    if (this.content?.json !== snap.contentJson) {
      let json: unknown = {};
      try {
        json = snap.contentJson ? JSON.parse(snap.contentJson) : {};
      } catch (e) {
        console.warn("[tospu] content.json is not valid JSON, using defaults", e);
        json = null;
      }
      const { content, error } = parseContent(json);
      if (error) console.warn("[tospu]", error);
      this.content = { json: snap.contentJson, source: new LocalContentSource(content) };
    }
    return this.content.source;
  }

  private rendererFor(pack: LoadedPack): SpriteRenderer {
    // Same pack and unchanged manifest → keep the mounted sprite.
    const key = JSON.stringify(pack.manifest);
    if (this.renderer?.key === key) return this.renderer.renderer;
    this.renderer?.renderer.destroy();
    const renderer = this.makeRenderer(pack);
    renderer.mount(this.spriteHost);
    this.renderer = { key, renderer };
    return renderer;
  }

  private reportHitRect() {
    const rects = [this.bubble.el, this.spriteHost].map((el) => el.getBoundingClientRect());
    const left = Math.min(...rects.map((r) => r.left));
    const top = Math.min(...rects.map((r) => r.top));
    const right = Math.max(...rects.map((r) => r.right));
    const bottom = Math.max(...rects.map((r) => r.bottom));
    void this.backend.setHitRect({ x: left, y: top, width: right - left, height: bottom - top });
  }
}
