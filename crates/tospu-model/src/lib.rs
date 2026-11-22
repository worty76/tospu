//! Pure serde types and logic shared between native backend and wasm UI.

pub mod anim;
pub mod content;
pub mod kind;
pub mod manifest;
pub mod settings;
pub mod time;
pub mod wire;

pub use anim::{typed, Effect, FrameAt, FrameClock, Phase, PopupTimeline, TYPE_MS_PER_GRAPHEME};
pub use content::{
    ContentError, ContentFile, Message, MessageDraft, MessageItem, MAX_AUTHOR_CHARS, MAX_MESSAGES,
    MAX_TEXT_CHARS, QUIET_TEXT,
};
pub use kind::MessageKind;
pub use manifest::{
    Anchor, AnimationName, AnimationSpec, Animations, Bubble, Frame, Manifest, RendererKind,
};
pub use settings::{DisplayTarget, IntervalSchedule, QuietHours, ScheduleMode, Settings, Slot};
pub use time::HhMm;
pub use wire::{
    MonitorInfo, OnboardingInput, PackSummary, PackView, Rect, SettingsView, ShowPayload,
};
