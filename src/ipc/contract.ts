import type { MessageKind } from "../domain/types";

// Mirrors the Rust side: src-tauri/src/commands.rs, state.rs and popup.rs.

export interface PackEntry {
  id: string;
  dir: string;
  manifestJson: string;
}

/** Only the settings fields the popup uses; Rust owns the full shape. */
export interface PopupSettings {
  activePack: string;
  displaySeconds: number;
  quietMode: boolean;
}

export interface Snapshot {
  settings: PopupSettings;
  packs: PackEntry[];
  contentJson: string | null;
  errors: string[];
}

export interface ShowPayload {
  kind: MessageKind;
  quiet: boolean;
}

export interface HitRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Everything the popup needs from the host. Faked in tests. */
export interface Backend {
  getSnapshot(): Promise<Snapshot>;
  setHitRect(rect: HitRect | null): Promise<void>;
  popupFinished(): Promise<void>;
  onShow(handler: (p: ShowPayload) => void): () => void;
  assetUrl(dir: string, file: string): string;
}

export const COMMANDS = {
  getSnapshot: "get_snapshot",
  setHitRect: "set_hit_rect",
  popupFinished: "popup_finished",
} as const;

export const EVENTS = {
  show: "tospu://show",
} as const;
