//! One-time migration from the legacy Yub data directory (`dev.yub.companion`)
//! to the Tospu data directory (`io.github.worty76.tospu`).

use std::fs;
use std::io;
use std::path::Path;

/// Copies `settings.json`, `content.json`, and `sprites/` from `old_dir` to
/// `new_dir` when `old_dir` exists and `new_dir/settings.json` does not.
/// Leaves `old_dir` untouched as a backup.
pub fn migrate_legacy_data(old_dir: &Path, new_dir: &Path) -> io::Result<bool> {
    if !old_dir.is_dir() || new_dir.join("settings.json").exists() {
        return Ok(false);
    }

    fs::create_dir_all(new_dir)?;

    for file in ["settings.json", "content.json"] {
        let src = old_dir.join(file);
        if src.is_file() {
            fs::copy(&src, new_dir.join(file))?;
        }
    }

    let old_sprites = old_dir.join("sprites");
    if old_sprites.is_dir() {
        copy_dir_all(&old_sprites, &new_dir.join("sprites"))?;
    }

    Ok(true)
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(path: impl AsRef<Path>) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn copies_settings_content_and_sprites_when_new_dir_is_fresh() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("dev.yub.companion");
        let new = root.path().join("io.github.worty76.tospu");
        fs::create_dir_all(old.join("sprites/fox")).unwrap();
        fs::write(old.join("settings.json"), r#"{"activePack":"fox"}"#).unwrap();
        fs::write(old.join("content.json"), r#"{"goals":["Ship"]}"#).unwrap();
        fs::write(old.join("sprites/fox/manifest.json"), r#"{"id":"fox"}"#).unwrap();

        assert!(migrate_legacy_data(&old, &new).unwrap());
        assert_eq!(
            read(new.join("settings.json")),
            read(old.join("settings.json"))
        );
        assert_eq!(
            read(new.join("content.json")),
            read(old.join("content.json"))
        );
        assert!(new.join("sprites/fox/manifest.json").is_file());
        assert!(old.join("settings.json").is_file()); // old kept as backup
    }

    #[test]
    fn does_nothing_when_new_dir_already_has_settings() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("dev.yub.companion");
        let new = root.path().join("io.github.worty76.tospu");
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(old.join("settings.json"), r#"{"activePack":"old"}"#).unwrap();
        fs::write(new.join("settings.json"), r#"{"activePack":"new"}"#).unwrap();

        assert!(!migrate_legacy_data(&old, &new).unwrap());
        assert_eq!(read(new.join("settings.json")), r#"{"activePack":"new"}"#);
    }

    #[test]
    fn does_nothing_when_old_dir_is_missing() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("dev.yub.companion");
        let new = root.path().join("io.github.worty76.tospu");
        assert!(!migrate_legacy_data(&missing, &new).unwrap());
    }
}
