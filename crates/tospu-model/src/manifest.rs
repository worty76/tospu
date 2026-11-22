use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::kind::MessageKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnimationName {
    Idle,
    Enter,
    Cheer,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RendererKind {
    #[serde(rename = "spritesheet")]
    SpriteSheet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    #[default]
    Top,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationSpec {
    pub sheet: String,
    pub frames: u32,
    pub fps: f32,
    #[serde(default)]
    pub r#loop: bool,
}

fn default_display_height() -> f32 {
    120.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_display_height")]
    pub display_height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Animations {
    pub idle: AnimationSpec,
    #[serde(default)]
    pub enter: Option<AnimationSpec>,
    #[serde(default)]
    pub cheer: Option<AnimationSpec>,
    #[serde(default)]
    pub exit: Option<AnimationSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bubble {
    #[serde(default)]
    pub anchor: Anchor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    pub renderer: RendererKind,
    pub frame: Frame,
    #[serde(default)]
    pub pixelated: bool,
    pub animations: Animations,
    #[serde(default)]
    pub moods: HashMap<MessageKind, AnimationName>,
    #[serde(default)]
    pub bubble: Bubble,
}

impl Manifest {
    pub fn parse(json: &str) -> Result<Manifest, String> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("manifest: invalid JSON ({e})"))?;

        let obj = value
            .as_object()
            .ok_or_else(|| "manifest: expected a JSON object".to_string())?;

        // Check required nested structures up front so missing fields report camelCase paths cleanly.
        if !obj.contains_key("schemaVersion") {
            return Err("schemaVersion: missing field".into());
        }
        if !obj.contains_key("id") {
            return Err("id: missing field".into());
        }
        if !obj.contains_key("name") {
            return Err("name: missing field".into());
        }
        match obj.get("renderer") {
            None => return Err("renderer: missing field".into()),
            Some(v) if v.as_str() != Some("spritesheet") => {
                return Err(format!("renderer: unsupported renderer {v}"));
            }
            _ => {}
        }
        if !obj.contains_key("frame") {
            return Err("frame: missing field".into());
        }
        match obj.get("animations").and_then(|v| v.as_object()) {
            None => return Err("animations: missing or not an object".into()),
            Some(anims) if !anims.contains_key("idle") => {
                return Err("animations.idle: missing field".into());
            }
            _ => {}
        }

        let manifest: Manifest = serde_json::from_value(value).map_err(format_serde_error)?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("schemaVersion: must be 1".into());
        }
        if !is_valid_id(&self.id) {
            return Err("id: use lowercase letters, digits and dashes".into());
        }
        if self.name.trim().is_empty() {
            return Err("name: must not be empty".into());
        }
        if !self.frame.width.is_finite() || self.frame.width <= 0.0 {
            return Err("frame.width: must be greater than 0".into());
        }
        if !self.frame.height.is_finite() || self.frame.height <= 0.0 {
            return Err("frame.height: must be greater than 0".into());
        }
        if !self.frame.display_height.is_finite() || self.frame.display_height <= 0.0 {
            return Err("frame.displayHeight: must be greater than 0".into());
        }

        validate_anim("idle", &self.animations.idle)?;
        if let Some(a) = &self.animations.enter {
            validate_anim("enter", a)?;
        }
        if let Some(a) = &self.animations.cheer {
            validate_anim("cheer", a)?;
        }
        if let Some(a) = &self.animations.exit {
            validate_anim("exit", a)?;
        }

        Ok(())
    }

    pub fn resolve_animation(&self, name: AnimationName) -> &AnimationSpec {
        match name {
            AnimationName::Idle => &self.animations.idle,
            AnimationName::Enter => self
                .animations
                .enter
                .as_ref()
                .unwrap_or(&self.animations.idle),
            AnimationName::Cheer => self
                .animations
                .cheer
                .as_ref()
                .unwrap_or(&self.animations.idle),
            AnimationName::Exit => self
                .animations
                .exit
                .as_ref()
                .unwrap_or(&self.animations.idle),
        }
    }

    pub fn animation_for_kind(&self, kind: MessageKind) -> AnimationName {
        self.moods
            .get(&kind)
            .copied()
            .unwrap_or(AnimationName::Idle)
    }

    pub fn display_size(&self) -> (f32, f32) {
        let h = self.frame.display_height;
        let w = ((self.frame.width / self.frame.height) * h).round();
        (w, h)
    }

    /// All distinct sheet filenames referenced by this manifest.
    pub fn sheet_names(&self) -> Vec<&str> {
        let mut names = vec![self.animations.idle.sheet.as_str()];
        for spec in [
            &self.animations.enter,
            &self.animations.cheer,
            &self.animations.exit,
        ]
        .into_iter()
        .flatten()
        {
            if !names.contains(&spec.sheet.as_str()) {
                names.push(spec.sheet.as_str());
            }
        }
        names
    }
}

fn format_serde_error(e: serde_json::Error) -> String {
    let msg = e.to_string();
    if let Some(rest) = msg.strip_prefix("missing field `") {
        if let Some(field) = rest.split('`').next() {
            return format!("{field}: missing field");
        }
    }
    format!("manifest: {msg}")
}

fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_valid_sheet(sheet: &str) -> bool {
    if sheet.contains("..") {
        return false;
    }
    let Some((stem, ext)) = sheet.rsplit_once('.') else {
        return false;
    };
    if stem.is_empty() {
        return false;
    }
    let ext_ok = matches!(ext.to_ascii_lowercase().as_str(), "png" | "webp" | "gif");
    ext_ok
        && stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn validate_anim(name: &str, spec: &AnimationSpec) -> Result<(), String> {
    if !is_valid_sheet(&spec.sheet) {
        return Err(format!(
            "animations.{name}.sheet: must be a file name inside the pack folder"
        ));
    }
    if spec.frames == 0 {
        return Err(format!("animations.{name}.frames: must be greater than 0"));
    }
    if !spec.fps.is_finite() || spec.fps <= 0.0 || spec.fps > 60.0 {
        return Err(format!("animations.{name}.fps: must be between 0 and 60"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"{
        "schemaVersion": 1,
        "id": "blob",
        "name": "Blob",
        "renderer": "spritesheet",
        "frame": { "width": 64, "height": 64 },
        "animations": { "idle": { "sheet": "idle.png", "frames": 4, "fps": 8, "loop": true } }
    }"#;

    #[test]
    fn accepts_minimal_pack_with_only_idle() {
        let m = Manifest::parse(MINIMAL).unwrap();
        assert_eq!(m.frame.display_height, 120.0);
        assert!(!m.pixelated);
        assert_eq!(m.bubble.anchor, Anchor::Top);
    }

    #[test]
    fn accepts_bundled_mochi_manifest() {
        let raw = include_str!("../../../sprites/mochi/manifest.json");
        assert!(Manifest::parse(raw).is_ok());
    }

    #[test]
    fn rejects_missing_idle_naming_the_field() {
        let raw = r#"{
            "schemaVersion": 1,
            "id": "blob",
            "name": "Blob",
            "renderer": "spritesheet",
            "frame": { "width": 64, "height": 64 },
            "animations": { "cheer": { "sheet": "idle.png", "frames": 4, "fps": 8 } }
        }"#;
        let err = Manifest::parse(raw).unwrap_err();
        assert!(err.contains("animations.idle"), "got: {err}");
    }

    #[test]
    fn rejects_ids_that_are_not_folder_safe() {
        let raw = MINIMAL.replace("\"blob\"", "\"Mochi Cat\"");
        let err = Manifest::parse(&raw).unwrap_err();
        assert!(err.starts_with("id:"), "got: {err}");
    }

    #[test]
    fn rejects_sheet_paths_outside_the_pack() {
        let raw = MINIMAL.replace("\"idle.png\"", "\"../x.png\"");
        let err = Manifest::parse(&raw).unwrap_err();
        assert!(err.contains("sheet"), "got: {err}");
    }

    #[test]
    fn rejects_unknown_renderer() {
        let raw = MINIMAL.replace("\"spritesheet\"", "\"lottie\"");
        let err = Manifest::parse(&raw).unwrap_err();
        assert!(err.contains("renderer"), "got: {err}");
    }

    #[test]
    fn falls_back_to_idle_when_animation_missing() {
        let m = Manifest::parse(MINIMAL).unwrap();
        assert_eq!(
            m.resolve_animation(AnimationName::Cheer),
            &m.animations.idle
        );
        assert_eq!(m.resolve_animation(AnimationName::Exit), &m.animations.idle);
    }

    #[test]
    fn mood_map_defaults_to_idle() {
        let raw = r#"{
            "schemaVersion": 1,
            "id": "blob",
            "name": "Blob",
            "renderer": "spritesheet",
            "frame": { "width": 64, "height": 64 },
            "animations": { "idle": { "sheet": "idle.png", "frames": 4, "fps": 8, "loop": true } },
            "moods": { "goal": "cheer" }
        }"#;
        let m = Manifest::parse(raw).unwrap();
        assert_eq!(
            m.animation_for_kind(MessageKind::Goal),
            AnimationName::Cheer
        );
        assert_eq!(
            m.animation_for_kind(MessageKind::Quote),
            AnimationName::Idle
        );
    }

    #[test]
    fn display_size_keeps_aspect() {
        let raw = include_str!("../../../sprites/mochi/manifest.json");
        let m = Manifest::parse(raw).unwrap();
        assert_eq!(m.display_size(), (140.0, 120.0));
    }
}
