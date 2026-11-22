//! Pure, GUI-free logic for Tospu. The Tauri shell wraps this crate; nothing
//! here knows about windows, trays or operating systems.

pub use tospu_model as model;

pub mod fsutil;
pub mod legacy;
pub mod packs;
pub mod scheduler;
pub mod settings_store;

pub use packs::{discover_packs, BrokenPack, Pack, PackScan};
pub use scheduler::{next_fire, Fire};
pub use tospu_model::{HhMm, Manifest, MessageKind, QuietHours, Settings, Slot};
