import type { AnimationName } from "../domain/types";
import { displaySize, resolveAnimation } from "./manifest";
import { registerRenderer, type LoadedPack, type SpriteRenderer } from "./renderer";

/** Plays horizontal sprite strips by stepping `background-position`. */
export class SpriteSheetRenderer implements SpriteRenderer {
  private el: HTMLDivElement | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;
  private finish: (() => void) | null = null;

  constructor(private readonly pack: LoadedPack) {}

  mount(host: HTMLElement): void {
    const { width, height } = displaySize(this.pack.manifest);
    const el = document.createElement("div");
    el.className = "sprite";
    el.style.width = `${width}px`;
    el.style.height = `${height}px`;
    el.style.backgroundRepeat = "no-repeat";
    if (this.pack.manifest.pixelated) el.style.imageRendering = "pixelated";
    host.appendChild(el);
    this.el = el;
  }

  play(name: AnimationName): Promise<void> {
    this.stop();
    const el = this.el;
    if (!el) return Promise.resolve();

    const spec = resolveAnimation(this.pack.manifest, name);
    const { width, height } = displaySize(this.pack.manifest);
    el.style.backgroundImage = `url("${this.pack.assetUrl(spec.sheet)}")`;
    el.style.backgroundSize = `${width * spec.frames}px ${height}px`;

    let frame = 0;
    const show = () => (el.style.backgroundPosition = `-${frame * width}px 0px`);
    show();

    return new Promise<void>((resolve) => {
      this.finish = resolve;
      this.timer = setInterval(() => {
        frame += 1;
        if (frame < spec.frames) return show();
        if (spec.loop) {
          frame = 0;
          show();
          // A loop "completes" each cycle; callers decide whether to replay.
          this.resolvePending();
          return;
        }
        this.clearTimer();
        this.resolvePending();
      }, 1000 / spec.fps);
    });
  }

  stop(): void {
    this.clearTimer();
    this.resolvePending();
  }

  destroy(): void {
    this.stop();
    this.el?.remove();
    this.el = null;
  }

  private clearTimer() {
    if (this.timer !== null) clearInterval(this.timer);
    this.timer = null;
  }

  private resolvePending() {
    const f = this.finish;
    this.finish = null;
    f?.();
  }
}

registerRenderer("spritesheet", (pack) => new SpriteSheetRenderer(pack));
