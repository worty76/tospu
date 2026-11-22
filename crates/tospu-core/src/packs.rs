//! Finds and validates sprite-pack folders on disk.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tospu_model::Manifest;

#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    pub id: String,
    pub dir: PathBuf,
    pub manifest: Manifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenPack {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PackScan {
    pub packs: Vec<Pack>,
    pub broken: Vec<BrokenPack>,
    pub errors: Vec<String>,
}

impl PackScan {
    /// Returns the active pack if valid, otherwise the first valid pack.
    pub fn choose(&self, active: &str) -> Option<&Pack> {
        self.packs
            .iter()
            .find(|p| p.id == active)
            .or_else(|| self.packs.first())
    }
}

/// Scan each root for `<root>/<id>/manifest.json`. Roots are listed from
/// lowest to highest priority: a later root's pack replaces an earlier one
/// with the same id, so a user pack can override a bundled one. Missing
/// roots are fine. Results are sorted by id.
pub fn discover_packs(roots: &[PathBuf]) -> PackScan {
    let mut found: BTreeMap<String, Result<Pack, BrokenPack>> = BTreeMap::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let dir = entry.path();
            let manifest_path = dir.join("manifest.json");
            if !dir.is_dir() || !manifest_path.is_file() {
                continue;
            }
            let Some(id) = folder_id(&dir) else { continue };
            let result = match fs::read_to_string(&manifest_path) {
                Ok(raw) => match Manifest::parse(&raw) {
                    Ok(manifest) if manifest.id == id => Ok(Pack {
                        id: id.clone(),
                        dir,
                        manifest,
                    }),
                    Ok(manifest) => Err(BrokenPack {
                        id: id.clone(),
                        reason: format!(
                            "id: manifest id \"{}\" does not match folder \"{}\"",
                            manifest.id, id
                        ),
                    }),
                    Err(reason) => Err(BrokenPack {
                        id: id.clone(),
                        reason,
                    }),
                },
                Err(e) => Err(BrokenPack {
                    id: id.clone(),
                    reason: format!("manifest.json: {e}"),
                }),
            };
            found.insert(id, result);
        }
    }

    let mut packs = Vec::new();
    let mut broken = Vec::new();
    let mut errors = Vec::new();
    for entry in found.into_values() {
        match entry {
            Ok(pack) => packs.push(pack),
            Err(b) => {
                errors.push(format!("sprite \"{}\": {}", b.id, b.reason));
                broken.push(b);
            }
        }
    }
    PackScan {
        packs,
        broken,
        errors,
    }
}

fn folder_id(dir: &Path) -> Option<String> {
    let name = dir.file_name()?.to_str()?;
    (!name.starts_with('.')).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest(id: &str, name: &str) -> String {
        format!(
            r#"{{
                "schemaVersion": 1,
                "id": "{id}",
                "name": "{name}",
                "renderer": "spritesheet",
                "frame": {{ "width": 64, "height": 64 }},
                "animations": {{ "idle": {{ "sheet": "idle.png", "frames": 4, "fps": 8, "loop": true }} }}
            }}"#
        )
    }

    fn pack(root: &Path, id: &str, body: &str) {
        let d = root.join(id);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("manifest.json"), body).unwrap();
    }

    #[test]
    fn finds_packs_sorted_and_ignores_folders_without_manifest() {
        let root = tempfile::tempdir().unwrap();
        pack(root.path(), "zed", &valid_manifest("zed", "Zed"));
        pack(root.path(), "mochi", &valid_manifest("mochi", "Mochi"));
        fs::create_dir_all(root.path().join("not-a-pack")).unwrap();
        fs::write(root.path().join("stray.json"), "{}").unwrap();

        let scan = discover_packs(&[root.path().to_path_buf()]);
        let ids: Vec<_> = scan.packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["mochi", "zed"]);
        assert_eq!(scan.packs[0].manifest.name, "Mochi");
        assert!(scan.broken.is_empty());
        assert!(scan.errors.is_empty());
    }

    #[test]
    fn later_root_overrides_same_id() {
        let bundled = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        pack(bundled.path(), "mochi", &valid_manifest("mochi", "Bundled"));
        pack(user.path(), "mochi", &valid_manifest("mochi", "User"));
        let scan = discover_packs(&[bundled.path().to_path_buf(), user.path().to_path_buf()]);
        assert_eq!(scan.packs.len(), 1);
        assert_eq!(scan.packs[0].manifest.name, "User");
        assert_eq!(scan.packs[0].dir, user.path().join("mochi"));
    }

    #[test]
    fn missing_roots_are_not_errors() {
        let scan = discover_packs(&[PathBuf::from("/definitely/not/here")]);
        assert_eq!(scan, PackScan::default());
    }

    #[test]
    fn invalid_manifest_goes_to_broken_with_reason() {
        let root = tempfile::tempdir().unwrap();
        pack(root.path(), "bad", "{ not json");
        let scan = discover_packs(&[root.path().to_path_buf()]);
        assert!(scan.packs.is_empty());
        assert_eq!(scan.broken.len(), 1);
        assert_eq!(scan.broken[0].id, "bad");
        assert!(!scan.broken[0].reason.is_empty());
        assert_eq!(scan.errors.len(), 1);
    }

    #[test]
    fn manifest_id_must_match_folder() {
        let root = tempfile::tempdir().unwrap();
        pack(root.path(), "fox", &valid_manifest("mochi", "Mochi"));
        let scan = discover_packs(&[root.path().to_path_buf()]);
        assert!(scan.packs.is_empty());
        assert_eq!(scan.broken.len(), 1);
        assert_eq!(scan.broken[0].id, "fox");
        assert!(scan.broken[0].reason.contains("id:"));
    }

    #[test]
    fn choose_falls_back_to_first_valid_pack() {
        let root = tempfile::tempdir().unwrap();
        pack(root.path(), "alpha", &valid_manifest("alpha", "Alpha"));
        pack(root.path(), "beta", &valid_manifest("beta", "Beta"));
        let scan = discover_packs(&[root.path().to_path_buf()]);
        assert_eq!(scan.choose("beta").unwrap().id, "beta");
        assert_eq!(scan.choose("missing").unwrap().id, "alpha");
    }
}
