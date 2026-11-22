//! Pure, GUI-free logic for Tospu. The Tauri shell wraps this crate; nothing
//! here knows about windows, trays or operating systems.

pub mod legacy;
pub mod packs;
pub mod scheduler;
pub mod settings;

pub use packs::{discover_packs, PackEntry, PackScan};
pub use scheduler::{next_fire, Fire};
pub use settings::{HhMm, MessageKind, QuietHours, Settings, Slot};
