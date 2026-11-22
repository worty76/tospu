//! Picks which message to show next without repeating recent ones.

use std::collections::{HashMap, VecDeque};

use tospu_model::{ContentFile, Message, MessageItem, MessageKind};

pub const FALLBACK_LINE: &str = "You're doing better than you think.";

pub struct Picker {
    history: usize,
    recent: HashMap<MessageKind, VecDeque<String>>,
}

impl Picker {
    pub fn new(history: usize) -> Self {
        Self {
            history,
            recent: HashMap::new(),
        }
    }

    pub fn next(
        &mut self,
        kind: MessageKind,
        content: &ContentFile,
        rand_index: &mut dyn FnMut(usize) -> usize,
    ) -> Message {
        let mut order = Vec::with_capacity(MessageKind::ALL.len());
        order.push(kind);
        for k in MessageKind::ALL {
            if k != kind {
                order.push(k);
            }
        }

        for candidate_kind in order {
            let enabled: Vec<&MessageItem> = content
                .messages
                .iter()
                .filter(|m| m.enabled && m.kind == candidate_kind)
                .collect();
            if enabled.is_empty() {
                continue;
            }
            return self.pick_from(candidate_kind, &enabled, rand_index);
        }

        Message {
            kind,
            text: FALLBACK_LINE.into(),
            author: None,
        }
    }

    fn pick_from(
        &mut self,
        kind: MessageKind,
        enabled: &[&MessageItem],
        rand_index: &mut dyn FnMut(usize) -> usize,
    ) -> Message {
        let recent = self.recent.entry(kind).or_default();
        let fresh: Vec<&MessageItem> = enabled
            .iter()
            .copied()
            .filter(|m| !recent.iter().any(|id| id == &m.id))
            .collect();
        let pool = if fresh.is_empty() { enabled } else { &fresh };

        let idx = rand_index(pool.len()) % pool.len();
        let chosen = pool[idx];

        let keep = self.history.min(enabled.len().saturating_sub(1));
        if keep == 0 {
            recent.clear();
        } else {
            recent.push_back(chosen.id.clone());
            while recent.len() > keep {
                recent.pop_front();
            }
        }

        format_message(chosen)
    }
}

fn format_message(item: &MessageItem) -> Message {
    match item.kind {
        MessageKind::Goal => Message {
            kind: item.kind,
            text: format!("Big goal: {}", item.text),
            author: None,
        },
        MessageKind::Gratitude => Message {
            kind: item.kind,
            text: format!("Grateful for: {}", item.text),
            author: None,
        },
        MessageKind::Quote => Message {
            kind: item.kind,
            text: item.text.clone(),
            author: item.author.clone(),
        },
        MessageKind::Proud => Message {
            kind: item.kind,
            text: item.text.clone(),
            author: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn item(
        id: &str,
        kind: MessageKind,
        text: &str,
        author: Option<&str>,
        enabled: bool,
    ) -> MessageItem {
        MessageItem {
            id: id.into(),
            kind,
            text: text.into(),
            author: author.map(Into::into),
            enabled,
        }
    }

    #[test]
    fn goal_and_gratitude_get_prefixes() {
        let f = ContentFile {
            version: 2,
            messages: vec![
                item("1", MessageKind::Goal, "A", None, true),
                item("2", MessageKind::Gratitude, "g", None, true),
            ],
        };
        let mut p = Picker::new(3);
        assert_eq!(
            p.next(MessageKind::Goal, &f, &mut |_| 0).text,
            "Big goal: A"
        );
        assert_eq!(
            p.next(MessageKind::Gratitude, &f, &mut |_| 0).text,
            "Grateful for: g"
        );
    }

    #[test]
    fn quotes_keep_author() {
        let f = ContentFile {
            version: 2,
            messages: vec![item(
                "1",
                MessageKind::Quote,
                "Stay calm",
                Some("Seneca"),
                true,
            )],
        };
        let mut p = Picker::new(3);
        let m = p.next(MessageKind::Quote, &f, &mut |_| 0);
        assert_eq!(m.text, "Stay calm");
        assert_eq!(m.author.as_deref(), Some("Seneca"));
    }

    #[test]
    fn does_not_repeat_within_history() {
        let f = ContentFile {
            version: 2,
            messages: vec![
                item("q1", MessageKind::Quote, "1", None, true),
                item("q2", MessageKind::Quote, "2", None, true),
                item("q3", MessageKind::Quote, "3", None, true),
                item("q4", MessageKind::Quote, "4", None, true),
            ],
        };
        let mut p = Picker::new(3);
        let picks: Vec<_> = (0..8)
            .map(|_| p.next(MessageKind::Quote, &f, &mut |_| 0).text)
            .collect();
        for w in picks.windows(4) {
            let unique: HashSet<_> = w.iter().collect();
            assert_eq!(unique.len(), 4, "repeated inside window: {w:?}");
        }
    }

    #[test]
    fn falls_back_to_another_kind_in_all_order_when_empty() {
        let f = ContentFile {
            version: 2,
            messages: vec![
                item("1", MessageKind::Gratitude, "family", None, true),
                item("2", MessageKind::Quote, "hi", None, true),
            ],
        };
        let mut p = Picker::new(3);
        let m = p.next(MessageKind::Proud, &f, &mut |_| 0);
        assert_eq!(m.kind, MessageKind::Quote);
        assert_eq!(m.text, "hi");
    }

    #[test]
    fn disabled_items_are_never_picked() {
        let f = ContentFile {
            version: 2,
            messages: vec![
                item("1", MessageKind::Goal, "Off", None, false),
                item("2", MessageKind::Goal, "On", None, true),
            ],
        };
        let mut p = Picker::new(3);
        for _ in 0..5 {
            assert_eq!(
                p.next(MessageKind::Goal, &f, &mut |_| 0).text,
                "Big goal: On"
            );
        }
    }

    #[test]
    fn returns_fallback_line_when_everything_is_empty_or_disabled() {
        let empty = ContentFile::default();
        let mut p = Picker::new(3);
        let m = p.next(MessageKind::Goal, &empty, &mut |_| 0);
        assert_eq!(m.text, FALLBACK_LINE);

        let disabled = ContentFile {
            version: 2,
            messages: vec![item("1", MessageKind::Goal, "Off", None, false)],
        };
        let m2 = p.next(MessageKind::Goal, &disabled, &mut |_| 0);
        assert_eq!(m2.text, FALLBACK_LINE);
    }
}
