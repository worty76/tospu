import type { AnimationName } from "../domain/types";
import type { SpriteManifest } from "./manifest";

/** A validated pack plus a way to turn its file names into loadable URLs. */
export interface LoadedPack {
  manifest: SpriteManifest;
  assetUrl(file: string): string;
}

/**
 * How a character moves on screen. The popup only talks to this interface,
 * so a new format (GIF, Lottie, Live2D...) is a new class plus one
 * `registerRenderer` call.
 */
export interface SpriteRenderer {
  mount(host: HTMLElement): void;
  /** Resolves when a one-shot animation ends, or after one cycle of a loop. */
  play(name: AnimationName): Promise<void>;
  stop(): void;
  destroy(): void;
}

export type RendererFactory = (pack: LoadedPack) => SpriteRenderer;

export class UnknownRendererError extends Error {
  constructor(kind: string) {
    super(`No renderer registered for "${kind}"`);
    this.name = "UnknownRendererError";
  }
}

const factories = new Map<string, RendererFactory>();

export function registerRenderer(kind: string, factory: RendererFactory): void {
  factories.set(kind, factory);
}

export function createRenderer(pack: LoadedPack): SpriteRenderer {
  const factory = factories.get(pack.manifest.renderer);
  if (!factory) throw new UnknownRendererError(pack.manifest.renderer);
  return factory(pack);
}
