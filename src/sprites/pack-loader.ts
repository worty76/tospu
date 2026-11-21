import type { PackEntry } from "../ipc/contract";
import { parseManifest } from "./manifest";
import type { LoadedPack } from "./renderer";

export interface PackLoadResult {
  packs: Map<string, LoadedPack>;
  errors: string[];
}

/** Validate every discovered pack; broken ones are skipped with a reason. */
export function loadPacks(entries: PackEntry[], assetUrl: (dir: string, file: string) => string): PackLoadResult {
  const packs = new Map<string, LoadedPack>();
  const errors: string[] = [];
  for (const entry of entries) {
    let json: unknown;
    try {
      json = JSON.parse(entry.manifestJson);
    } catch (e) {
      errors.push(`sprite "${entry.id}": manifest.json is not valid JSON (${(e as Error).message})`);
      continue;
    }
    const r = parseManifest(json);
    if (!r.ok) {
      errors.push(`sprite "${entry.id}": ${r.error}`);
      continue;
    }
    if (r.manifest.id !== entry.id) {
      errors.push(`sprite "${entry.id}": manifest id "${r.manifest.id}" must match the folder name`);
      continue;
    }
    packs.set(entry.id, { manifest: r.manifest, assetUrl: (file) => assetUrl(entry.dir, file) });
  }
  return { packs, errors };
}

/** The active pack, or the first valid one if it's missing or broken. */
export function choosePack(packs: Map<string, LoadedPack>, activeId: string): LoadedPack | undefined {
  return packs.get(activeId) ?? packs.values().next().value;
}
