import { z } from "zod";
import { ANIMATION_NAMES, MESSAGE_KINDS, type AnimationName, type MessageKind } from "../domain/types";

const animationSpec = z.object({
  sheet: z.string().regex(/^[\w.-]+\.(png|webp|gif)$/i, "must be a file name inside the pack folder"),
  frames: z.number().int().positive(),
  fps: z.number().positive().max(60),
  loop: z.boolean().default(false),
});

const animationName = z.enum(ANIMATION_NAMES);

const manifestSchema = z.object({
  schemaVersion: z.literal(1),
  id: z.string().regex(/^[a-z0-9][a-z0-9-]*$/, "use lowercase letters, digits and dashes"),
  name: z.string().min(1),
  author: z.string().optional(),
  license: z.string().optional(),
  renderer: z.string().min(1),
  frame: z.object({
    width: z.number().positive(),
    height: z.number().positive(),
    /** On-screen height in CSS pixels; width follows the frame's aspect ratio. */
    displayHeight: z.number().positive().default(120),
  }),
  pixelated: z.boolean().default(false),
  animations: z
    .object({ idle: animationSpec })
    .extend(Object.fromEntries(ANIMATION_NAMES.filter((n) => n !== "idle").map((n) => [n, animationSpec.optional()]))),
  moods: z.record(z.enum(MESSAGE_KINDS), animationName).default({}),
  bubble: z.object({ anchor: z.enum(["top", "left", "right"]).default("top") }).default({}),
});

export type AnimationSpec = z.infer<typeof animationSpec>;
export type SpriteManifest = z.infer<typeof manifestSchema> & {
  animations: { idle: AnimationSpec } & Partial<Record<AnimationName, AnimationSpec>>;
};

export type ManifestResult = { ok: true; manifest: SpriteManifest } | { ok: false; error: string };

export function parseManifest(json: unknown): ManifestResult {
  const r = manifestSchema.safeParse(json);
  if (r.success) return { ok: true, manifest: r.data as SpriteManifest };
  const first = r.error.issues[0];
  const where = first?.path.join(".") || "manifest";
  return { ok: false, error: `${where}: ${first?.message ?? "invalid"}` };
}

/** The spec for `name`, or `idle` when the pack doesn't provide it. */
export function resolveAnimation(m: SpriteManifest, name: AnimationName): AnimationSpec {
  return m.animations[name] ?? m.animations.idle;
}

/** Which animation plays while a message of `kind` is on screen. */
export function animationForKind(m: SpriteManifest, kind: MessageKind): AnimationName {
  return m.moods[kind] ?? "idle";
}

/** On-screen size of one frame in CSS pixels. */
export function displaySize(m: SpriteManifest): { width: number; height: number } {
  const height = m.frame.displayHeight;
  return { width: Math.round((m.frame.width / m.frame.height) * height), height };
}
