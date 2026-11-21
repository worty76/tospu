import "./sprites/builtin";
import "./popup/styles.css";
import type { Backend } from "./ipc/contract";
import { PopupController } from "./popup/popup-controller";

async function backend(): Promise<Backend> {
  if ("__TAURI_INTERNALS__" in window) return (await import("./ipc/tauri-backend")).tauriBackend;
  // Plain browser (`npm run dev`): preview popups with the bundled sprites.
  return (await import("./dev/demo-backend")).createDemoBackend();
}

void backend().then((b) => {
  const controller = new PopupController({
    stage: document.getElementById("stage")!,
    backend: b,
    reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
  });
  b.onShow(({ kind, quiet }) => void controller.show(kind, quiet));
});
