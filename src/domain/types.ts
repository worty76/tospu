/** The four kinds of nudges Tospu can deliver. */
export const MESSAGE_KINDS = ["goal", "quote", "gratitude", "proud"] as const;
export type MessageKind = (typeof MESSAGE_KINDS)[number];

export interface Message {
  kind: MessageKind;
  text: string;
  author?: string;
}

/** Animation slots a sprite pack can fill. Only `idle` is required. */
export const ANIMATION_NAMES = ["idle", "enter", "cheer", "exit"] as const;
export type AnimationName = (typeof ANIMATION_NAMES)[number];

export const QUIET_TEXT = "Keep going.";
