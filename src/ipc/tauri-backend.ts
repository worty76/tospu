import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { COMMANDS, EVENTS, type Backend, type ShowPayload, type Snapshot } from "./contract";

/** Wraps a promise-returning `listen` so callers get a sync unsubscribe. */
function subscribe<T>(event: string, handler: (payload: T) => void): () => void {
  const pending = listen<T>(event, (e) => handler(e.payload));
  return () => void pending.then((off) => off());
}

export const tauriBackend: Backend = {
  getSnapshot: () => invoke<Snapshot>(COMMANDS.getSnapshot),
  setHitRect: (rect) => invoke(COMMANDS.setHitRect, { rect }),
  popupFinished: () => invoke(COMMANDS.popupFinished),
  onShow: (handler) => subscribe<ShowPayload>(EVENTS.show, handler),
  assetUrl: (dir, file) => convertFileSrc(`${dir.replace(/[\\/]$/, "")}/${file}`),
};
