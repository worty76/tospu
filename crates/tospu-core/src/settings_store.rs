//! Reading and writing `settings.json` safely.

use std::fs;
use std::io;
use std::path::Path;

use tospu_model::Settings;

use crate::fsutil::atomic_write;

/// Load settings, creating the file with defaults if it doesn't exist.
/// A broken file is left untouched (so the user can fix it) and the
/// defaults are used, with the reason returned alongside.
pub fn load_or_create(path: &Path) -> (Settings, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(s) => (s, None),
            Err(e) => (Settings::default(), Some(format!("settings.json: {e}"))),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let s = Settings::default();
            let err = save(path, &s)
                .err()
                .map(|e| format!("could not create settings.json: {e}"));
            (s, err)
        }
        Err(e) => (Settings::default(), Some(format!("settings.json: {e}"))),
    }
}

pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    let text = serde_json::to_string_pretty(settings).expect("settings always serialize");
    atomic_write(path, (text + "\n").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_created_and_broken_file_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (s, err) = load_or_create(&path);
        assert_eq!((s, err), (Settings::default(), None));
        assert!(path.exists());

        fs::write(&path, "{ nope").unwrap();
        let (s, err) = load_or_create(&path);
        assert_eq!(s, Settings::default());
        assert!(err.unwrap().starts_with("settings.json"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ nope");
    }
}
