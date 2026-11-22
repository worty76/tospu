use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::kind::MessageKind;

pub const MAX_TEXT_CHARS: usize = 280;
pub const MAX_AUTHOR_CHARS: usize = 80;
pub const MAX_MESSAGES: usize = 500;
pub const QUIET_TEXT: &str = "Keep going.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageItem {
    pub id: String,
    pub kind: MessageKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDraft {
    pub kind: MessageKind,
    pub text: String,
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentFile {
    pub version: u32,
    #[serde(default)]
    pub messages: Vec<MessageItem>,
}

impl Default for ContentFile {
    fn default() -> Self {
        Self {
            version: 2,
            messages: Vec::new(),
        }
    }
}

/// A message ready to display in the popup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub kind: MessageKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentError {
    EmptyText,
    TextTooLong,
    AuthorTooLong,
    TooMany,
    NotFound(String),
    DuplicateId(String),
    UnsupportedVersion(u32),
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyText => write!(f, "Text cannot be empty."),
            Self::TextTooLong => write!(f, "Text can be at most {MAX_TEXT_CHARS} characters."),
            Self::AuthorTooLong => {
                write!(f, "Author can be at most {MAX_AUTHOR_CHARS} characters.")
            }
            Self::TooMany => write!(f, "You can save at most {MAX_MESSAGES} messages."),
            Self::NotFound(id) => write!(f, "Message \"{id}\" was not found."),
            Self::DuplicateId(id) => write!(f, "Duplicate message id \"{id}\"."),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported content version {v}."),
        }
    }
}

impl std::error::Error for ContentError {}

#[derive(Deserialize)]
#[serde(untagged)]
enum V1Goal {
    Text(String),
    Obj { title: String },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum V1Quote {
    Text(String),
    Obj {
        text: String,
        #[serde(default)]
        author: Option<String>,
    },
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct V1Content {
    goals: Vec<V1Goal>,
    quotes: Vec<V1Quote>,
    gratitude: Vec<String>,
    proud: Vec<String>,
}

fn normalize_author(author: Option<String>) -> Option<String> {
    author
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
}

fn validate_fields(text: &str, author: Option<&str>) -> Result<(), ContentError> {
    if text.is_empty() {
        return Err(ContentError::EmptyText);
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(ContentError::TextTooLong);
    }
    if let Some(a) = author {
        if a.chars().count() > MAX_AUTHOR_CHARS {
            return Err(ContentError::AuthorTooLong);
        }
    }
    Ok(())
}

impl ContentFile {
    pub fn parse(
        json: &str,
        new_id: &mut dyn FnMut() -> String,
    ) -> Result<(ContentFile, bool), String> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("invalid JSON ({e})"))?;
        let obj = value
            .as_object()
            .ok_or_else(|| "expected a JSON object".to_string())?;

        if let Some(ver_val) = obj.get("version") {
            let version = ver_val
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .ok_or_else(|| "version must be a positive integer".to_string())?;
            if version != 2 {
                return Err(ContentError::UnsupportedVersion(version).to_string());
            }
            let file: ContentFile =
                serde_json::from_value(value).map_err(|e| format!("invalid v2 content ({e})"))?;
            file.validate().map_err(|e| e.to_string())?;
            return Ok((file, false));
        }

        // Legacy v1 format
        let v1: V1Content =
            serde_json::from_value(value).map_err(|e| format!("invalid v1 content ({e})"))?;
        let mut file = ContentFile::default();

        for g in v1.goals {
            let text = match g {
                V1Goal::Text(t) => t,
                V1Goal::Obj { title } => title,
            };
            file.add(
                MessageDraft {
                    kind: MessageKind::Goal,
                    text,
                    author: None,
                },
                new_id(),
            )
            .map_err(|e| e.to_string())?;
        }
        for q in v1.quotes {
            let (text, author) = match q {
                V1Quote::Text(t) => (t, None),
                V1Quote::Obj { text, author } => (text, author),
            };
            file.add(
                MessageDraft {
                    kind: MessageKind::Quote,
                    text,
                    author,
                },
                new_id(),
            )
            .map_err(|e| e.to_string())?;
        }
        for g in v1.gratitude {
            file.add(
                MessageDraft {
                    kind: MessageKind::Gratitude,
                    text: g,
                    author: None,
                },
                new_id(),
            )
            .map_err(|e| e.to_string())?;
        }
        for p in v1.proud {
            file.add(
                MessageDraft {
                    kind: MessageKind::Proud,
                    text: p,
                    author: None,
                },
                new_id(),
            )
            .map_err(|e| e.to_string())?;
        }

        file.validate().map_err(|e| e.to_string())?;
        Ok((file, true))
    }

    pub fn add(&mut self, draft: MessageDraft, id: String) -> Result<&MessageItem, ContentError> {
        if self.messages.len() >= MAX_MESSAGES {
            return Err(ContentError::TooMany);
        }
        if self.messages.iter().any(|m| m.id == id) {
            return Err(ContentError::DuplicateId(id));
        }
        let text = draft.text.trim().to_string();
        let author = normalize_author(draft.author);
        validate_fields(&text, author.as_deref())?;

        self.messages.push(MessageItem {
            id,
            kind: draft.kind,
            text,
            author,
            enabled: true,
        });
        Ok(self.messages.last().expect("just pushed"))
    }

    pub fn update(&mut self, item: MessageItem) -> Result<&MessageItem, ContentError> {
        let Some(slot) = self.messages.iter_mut().find(|m| m.id == item.id) else {
            return Err(ContentError::NotFound(item.id));
        };
        let text = item.text.trim().to_string();
        let author = normalize_author(item.author);
        validate_fields(&text, author.as_deref())?;

        slot.kind = item.kind;
        slot.text = text;
        slot.author = author;
        slot.enabled = item.enabled;
        Ok(slot)
    }

    pub fn remove(&mut self, id: &str) -> Result<MessageItem, ContentError> {
        let Some(pos) = self.messages.iter().position(|m| m.id == id) else {
            return Err(ContentError::NotFound(id.to_string()));
        };
        Ok(self.messages.remove(pos))
    }

    pub fn validate(&self) -> Result<(), ContentError> {
        if self.version != 2 {
            return Err(ContentError::UnsupportedVersion(self.version));
        }
        if self.messages.len() > MAX_MESSAGES {
            return Err(ContentError::TooMany);
        }
        let mut seen = HashSet::new();
        for item in &self.messages {
            if !seen.insert(item.id.as_str()) {
                return Err(ContentError::DuplicateId(item.id.clone()));
            }
            validate_fields(
                item.text.trim(),
                item.author
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq_id() -> impl FnMut() -> String {
        let mut n = 0usize;
        move || {
            n += 1;
            format!("id-{n}")
        }
    }

    #[test]
    fn migrates_v1_all_four_lists() {
        let raw = r#"{"goals":[{"title":"A"},"B"],"quotes":["q",{"text":"t","author":"S"}],"gratitude":["g"],"proud":["p"]}"#;
        let (file, was_v1) = ContentFile::parse(raw, &mut seq_id()).unwrap();
        assert!(was_v1);
        assert_eq!(file.version, 2);
        assert_eq!(file.messages.len(), 6);
        let kinds: Vec<_> = file.messages.iter().map(|m| m.kind).collect();
        assert_eq!(
            kinds,
            vec![
                MessageKind::Goal,
                MessageKind::Goal,
                MessageKind::Quote,
                MessageKind::Quote,
                MessageKind::Gratitude,
                MessageKind::Proud,
            ]
        );
        let texts: Vec<_> = file.messages.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, vec!["A", "B", "q", "t", "g", "p"]);
        assert_eq!(file.messages[3].author.as_deref(), Some("S"));
        assert!(file.messages.iter().all(|m| m.enabled));
    }

    #[test]
    fn parses_v2_unchanged() {
        let raw = r#"{"version":2,"messages":[{"id":"a1","kind":"quote","text":"Hi","author":"Seneca","enabled":true}]}"#;
        let (file, was_v1) = ContentFile::parse(raw, &mut seq_id()).unwrap();
        assert!(!was_v1);
        assert_eq!(file.messages.len(), 1);
        assert_eq!(file.messages[0].id, "a1");
        assert_eq!(file.messages[0].author.as_deref(), Some("Seneca"));
    }

    #[test]
    fn rejects_version_3() {
        let raw = r#"{"version":3,"messages":[]}"#;
        assert!(ContentFile::parse(raw, &mut seq_id()).is_err());
    }

    #[test]
    fn add_trims_and_drops_empty_author() {
        let mut f = ContentFile::default();
        let item = f
            .add(
                MessageDraft {
                    kind: MessageKind::Quote,
                    text: "  Hello  ".into(),
                    author: Some("   ".into()),
                },
                "id1".into(),
            )
            .unwrap();
        assert_eq!(item.text, "Hello");
        assert_eq!(item.author, None);
        assert!(item.enabled);
    }

    #[test]
    fn add_rejects_empty_and_whitespace_text() {
        let mut f = ContentFile::default();
        for bad in ["", "   \n\t  "] {
            let res = f.add(
                MessageDraft {
                    kind: MessageKind::Goal,
                    text: bad.into(),
                    author: None,
                },
                "id1".into(),
            );
            assert_eq!(res, Err(ContentError::EmptyText));
        }
    }

    #[test]
    fn text_limit_counts_chars_not_bytes() {
        let mut f = ContentFile::default();
        let ok_text = "ộ".repeat(280);
        assert!(f
            .add(
                MessageDraft {
                    kind: MessageKind::Goal,
                    text: ok_text,
                    author: None
                },
                "id1".into()
            )
            .is_ok());

        let too_long = "ộ".repeat(281);
        let res = f.add(
            MessageDraft {
                kind: MessageKind::Goal,
                text: too_long,
                author: None,
            },
            "id2".into(),
        );
        assert_eq!(res, Err(ContentError::TextTooLong));
    }

    #[test]
    fn add_rejects_501st() {
        let mut f = ContentFile::default();
        for i in 0..500 {
            f.add(
                MessageDraft {
                    kind: MessageKind::Proud,
                    text: format!("m{i}"),
                    author: None,
                },
                format!("id-{i}"),
            )
            .unwrap();
        }
        let res = f.add(
            MessageDraft {
                kind: MessageKind::Proud,
                text: "overflow".into(),
                author: None,
            },
            "id-500".into(),
        );
        assert_eq!(res, Err(ContentError::TooMany));
    }

    #[test]
    fn update_unknown_id_is_not_found() {
        let mut f = ContentFile::default();
        let res = f.update(MessageItem {
            id: "nope".into(),
            kind: MessageKind::Goal,
            text: "Hi".into(),
            author: None,
            enabled: true,
        });
        assert_eq!(res, Err(ContentError::NotFound("nope".into())));
    }

    #[test]
    fn remove_returns_item() {
        let mut f = ContentFile::default();
        f.add(
            MessageDraft {
                kind: MessageKind::Goal,
                text: "Ship".into(),
                author: None,
            },
            "a".into(),
        )
        .unwrap();
        let removed = f.remove("a").unwrap();
        assert_eq!(removed.text, "Ship");
        assert!(f.messages.is_empty());
    }

    #[test]
    fn duplicate_ids_fail_validate() {
        let f = ContentFile {
            version: 2,
            messages: vec![
                MessageItem {
                    id: "dup".into(),
                    kind: MessageKind::Goal,
                    text: "A".into(),
                    author: None,
                    enabled: true,
                },
                MessageItem {
                    id: "dup".into(),
                    kind: MessageKind::Goal,
                    text: "B".into(),
                    author: None,
                    enabled: true,
                },
            ],
        };
        assert_eq!(f.validate(), Err(ContentError::DuplicateId("dup".into())));
    }
}
