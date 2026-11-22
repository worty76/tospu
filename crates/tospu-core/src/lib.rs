//! Pure, GUI-free logic for Tospu. The Tauri shell wraps this crate; nothing
//! here knows about windows, trays or operating systems.

pub use tospu_model as model;

pub mod content_store;
pub mod describe;
pub mod fsutil;
pub mod legacy;
pub mod packs;
pub mod picker;
pub mod scheduler;
pub mod settings_store;

pub use content_store::{new_id, ContentStore};
pub use describe::describe_next;
pub use packs::{discover_packs, BrokenPack, Pack, PackScan};
pub use picker::{Picker, FALLBACK_LINE};
pub use scheduler::{next_fire, Fire};
pub use tospu_model::{
    ContentError, ContentFile, DisplayTarget, HhMm, IntervalSchedule, Manifest, Message,
    MessageDraft, MessageItem, MessageKind, MonitorInfo, OnboardingInput, PackSummary, PackView,
    QuietHours, Rect, ScheduleMode, Settings, SettingsView, ShowPayload, Slot,
};
