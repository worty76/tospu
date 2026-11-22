//! Pure serde types and logic shared between native backend and wasm UI.

pub mod kind;
pub mod manifest;
pub mod settings;
pub mod time;

pub use kind::MessageKind;
pub use manifest::{
    Anchor, AnimationName, AnimationSpec, Animations, Bubble, Frame, Manifest, RendererKind,
};
pub use settings::{QuietHours, Settings, Slot};
pub use time::HhMm;
