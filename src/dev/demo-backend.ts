import defaults from "../../defaults/content.json";
import { MESSAGE_KINDS } from "../domain/types";
import type { Backend, PackEntry, ShowPayload } from "../ipc/contract";

// Browser-only stand-in for the Tauri backend, for designing sprites.
// URL options: ?pack=mochi-fluffy&quiet=1&seconds=4

const manifests = import.meta.glob("../../sprites/*/manifest.json", { eager: true, query: "?raw", import: "default" });

const packs: PackEntry[] = Object.entries(manifests).map(([path, json]) => {
  const id = path.split("/").at(-2)!;
  return { id, dir: `/sprites/${id}`, manifestJson: json as string };
});

export function createDemoBackend(): Backend {
  const params = new URLSearchParams(location.search);
  const quiet = params.has("quiet");
  let handler: ((p: ShowPayload) => void) | null = null;
  let n = 0;
  const next = () => handler?.({ kind: MESSAGE_KINDS[n++ % MESSAGE_KINDS.length]!, quiet });

  document.body.classList.add("demo");
  setTimeout(next, 500);

  return {
    getSnapshot: async () => ({
      settings: {
        activePack: params.get("pack") ?? "mochi",
        displaySeconds: Number(params.get("seconds") ?? 5),
        quietMode: quiet,
      },
      packs,
      contentJson: JSON.stringify(defaults),
      errors: [],
    }),
    setHitRect: async () => {},
    popupFinished: async () => void setTimeout(next, 1500),
    onShow: (h) => {
      handler = h;
      return () => (handler = null);
    },
    assetUrl: (dir, file) => `${dir}/${file}`,
  };
}
