use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Goal,
    Quote,
    Gratitude,
    Proud,
}

impl MessageKind {
    pub const ALL: [MessageKind; 4] = [Self::Goal, Self::Quote, Self::Gratitude, Self::Proud];
}
