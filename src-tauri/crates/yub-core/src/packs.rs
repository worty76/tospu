//! Finds sprite-pack folders on disk. Validation of the manifest itself lives
//! in the front end (one schema, one place); this module only discovers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackEntry {
    /// Folder name; the manifest's `id` must match it.
    pub id: String,
    pub dir: PathBuf,
    pub manifest_json: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct PackScan {
    pub packs: Vec<PackEntry>,
    pub errors: Vec<String>,
}

/// Scan each root for `<root>/<id>/manifest.json`. Roots are listed from
/// lowest to highest priority: a later root's pack replaces an earlier one
/// with the same id, so a user pack can override a bundled one. Missing
/// roots are fine. Results are sorted by id.
pub fn discover_packs(roots: &[PathBuf]) -> PackScan {
    let mut found = BTreeMap::new();
    let mut errors = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else { continue };
        for entry in entries.flatten() {
            let dir = entry.path();
            let manifest = dir.join("manifest.json");
            if !dir.is_dir() || !manifest.is_file() {
                continue;
            }
            let Some(id) = folder_id(&dir) else { continue };
            match fs::read_to_string(&manifest) {
                Ok(manifest_json) => {
                    found.insert(id.clone(), PackEntry { id, dir, manifest_json });
                }
                Err(e) => errors.push(format!("{}: {e}", manifest.display())),
            }
        }
    }
    PackScan { packs: found.into_values().collect(), errors }
}

fn folder_id(dir: &Path) -> Option<String> {
    let name = dir.file_name()?.to_str()?;
    (!name.starts_with('.')).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(root: &Path, id: &str, body: &str) {
        let d = root.join(id);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("manifest.json"), body).unwrap();
    }

    #[test]
    fn finds_packs_sorted_and_ignores_folders_without_manifest() {
        let root = tempfile::tempdir().unwrap();
        pack(root.path(), "zed", "{}");
        pack(root.path(), "mochi", "{\"id\":\"mochi\"}");
        fs::create_dir_all(root.path().join("not-a-pack")).unwrap();
        fs::write(root.path().join("stray.json"), "{}").unwrap();

        let scan = discover_packs(&[root.path().to_path_buf()]);
        let ids: Vec<_> = scan.packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["mochi", "zed"]);
        assert_eq!(scan.packs[0].manifest_json, "{\"id\":\"mochi\"}");
        assert!(scan.errors.is_empty());
    }

    #[test]
    fn later_root_overrides_same_id() {
        let bundled = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        pack(bundled.path(), "mochi", "\"bundled\"");
        pack(user.path(), "mochi", "\"user\"");
        let scan = discover_packs(&[bundled.path().to_path_buf(), user.path().to_path_buf()]);
        assert_eq!(scan.packs.len(), 1);
        assert_eq!(scan.packs[0].manifest_json, "\"user\"");
        assert_eq!(scan.packs[0].dir, user.path().join("mochi"));
    }

    #[test]
    fn missing_roots_are_not_errors() {
        let scan = discover_packs(&[PathBuf::from("/definitely/not/here")]);
        assert_eq!(scan, PackScan::default());
    }
}
