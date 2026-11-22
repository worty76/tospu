//! Safe persistence and v1->v2 migration for `content.json`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tospu_model::{ContentError, ContentFile};

use crate::fsutil::atomic_write;

/// Generate a 12-character lowercase hex id.
pub fn new_id() -> String {
    let val = fastrand::u64(..) & 0x0000_ffff_ffff_ffff;
    format!("{val:012x}")
}

pub struct ContentStore {
    path: PathBuf,
    defaults_json: String,
    file: ContentFile,
    problem: Option<String>,
}

impl ContentStore {
    pub fn open(path: &Path, defaults_json: &str) -> ContentStore {
        let mut store = ContentStore {
            path: path.to_path_buf(),
            defaults_json: defaults_json.to_string(),
            file: ContentFile::default(),
            problem: None,
        };
        store.reload();
        store
    }

    pub fn file(&self) -> &ContentFile {
        &self.file
    }

    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    pub fn reload(&mut self) {
        match fs::read_to_string(&self.path) {
            Ok(raw) => match ContentFile::parse(&raw, &mut new_id) {
                Ok((file, was_v1)) => {
                    if was_v1 {
                        if let Some(parent) = self.path.parent() {
                            let bak = parent.join("content.v1.json.bak");
                            if !bak.exists() {
                                if let Err(e) = atomic_write(&bak, raw.as_bytes()) {
                                    self.file = file;
                                    self.problem = Some(format!(
                                        "content.json: could not write v1 backup ({e})"
                                    ));
                                    return;
                                }
                            }
                        }
                        if let Err(e) = write_pretty(&self.path, &file) {
                            self.file = file;
                            self.problem =
                                Some(format!("content.json: could not save migrated v2 ({e})"));
                            return;
                        }
                    }
                    self.file = file;
                    self.problem = None;
                }
                Err(e) => {
                    self.file = ContentFile::default();
                    self.problem = Some(format!("content.json: {e}"));
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                match ContentFile::parse(&self.defaults_json, &mut new_id) {
                    Ok((file, _)) => {
                        let problem = write_pretty(&self.path, &file)
                            .err()
                            .map(|err| format!("could not create content.json: {err}"));
                        self.file = file;
                        self.problem = problem;
                    }
                    Err(err) => {
                        self.file = ContentFile::default();
                        self.problem = Some(format!("defaults/content.json: {err}"));
                    }
                }
            }
            Err(e) => {
                self.file = ContentFile::default();
                self.problem = Some(format!("content.json: {e}"));
            }
        }
    }

    pub fn change<T>(
        &mut self,
        f: impl FnOnce(&mut ContentFile) -> Result<T, ContentError>,
    ) -> Result<T, String> {
        if self.problem.is_some() {
            return Err("content.json could not be read. Fix it or click Reload.".into());
        }
        let mut next = self.file.clone();
        let result = f(&mut next).map_err(|e| e.to_string())?;
        next.validate().map_err(|e| e.to_string())?;
        write_pretty(&self.path, &next).map_err(|e| format!("could not save content.json: {e}"))?;
        self.file = next;
        Ok(result)
    }
}

fn write_pretty(path: &Path, file: &ContentFile) -> io::Result<()> {
    let text = serde_json::to_string_pretty(file).expect("content file always serializes");
    atomic_write(path, (text + "\n").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tospu_model::{MessageDraft, MessageKind};

    const DEFAULTS: &str =
        r#"{"version":2,"messages":[{"id":"d1","kind":"quote","text":"Hi","enabled":true}]}"#;

    #[test]
    fn missing_file_is_created_from_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.json");
        let store = ContentStore::open(&path, DEFAULTS);
        assert!(store.problem().is_none());
        assert_eq!(store.file().messages.len(), 1);
        assert!(path.is_file());
    }

    #[test]
    fn v1_file_is_migrated_and_backed_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.json");
        let v1 = r#"{"goals":["Ship Tospu"],"quotes":["Be kind"]}"#;
        fs::write(&path, v1).unwrap();

        let store = ContentStore::open(&path, DEFAULTS);
        assert!(store.problem().is_none());
        assert_eq!(store.file().messages.len(), 2);

        let bak = dir.path().join("content.v1.json.bak");
        assert_eq!(fs::read_to_string(&bak).unwrap(), v1);
        let written = fs::read_to_string(&path).unwrap();
        assert!(written.contains("\"version\": 2"));
    }

    #[test]
    fn second_v1_migration_preserves_existing_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.json");
        let bak = dir.path().join("content.v1.json.bak");
        fs::write(&bak, r#"{"goals":["Original backup"]}"#).unwrap();
        fs::write(&path, r#"{"goals":["Second v1"]}"#).unwrap();

        let store = ContentStore::open(&path, DEFAULTS);
        assert!(store.problem().is_none());
        assert_eq!(
            fs::read_to_string(&bak).unwrap(),
            r#"{"goals":["Original backup"]}"#
        );
    }

    #[test]
    fn broken_file_is_left_untouched_and_changes_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.json");
        fs::write(&path, "{ broken").unwrap();

        let mut store = ContentStore::open(&path, DEFAULTS);
        assert!(store.problem().is_some());
        assert!(store.file().messages.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ broken");

        let err = store
            .change(|f| {
                f.add(
                    MessageDraft {
                        kind: MessageKind::Goal,
                        text: "Test".into(),
                        author: None,
                    },
                    new_id(),
                )
                .map(|_| ())
            })
            .unwrap_err();
        assert!(err.contains("Reload"), "got: {err}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ broken");
    }

    #[test]
    fn failed_change_does_not_touch_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.json");
        let mut store = ContentStore::open(&path, DEFAULTS);
        let before = fs::read_to_string(&path).unwrap();

        let res = store.change(|f| {
            f.add(
                MessageDraft {
                    kind: MessageKind::Goal,
                    text: "   ".into(),
                    author: None,
                },
                new_id(),
            )
            .map(|_| ())
        });
        assert!(res.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn successful_change_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.json");
        let mut store = ContentStore::open(&path, DEFAULTS);

        store
            .change(|f| {
                f.add(
                    MessageDraft {
                        kind: MessageKind::Goal,
                        text: "Learn Rust".into(),
                        author: None,
                    },
                    new_id(),
                )
                .map(|_| ())
            })
            .unwrap();

        let reloaded = ContentStore::open(&path, DEFAULTS);
        assert_eq!(reloaded.file().messages.len(), 2);
        assert_eq!(reloaded.file().messages[1].text, "Learn Rust");
    }
}
