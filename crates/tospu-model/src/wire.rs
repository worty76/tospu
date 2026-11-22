use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::content::Message;
use crate::manifest::Manifest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackView {
    pub id: String,
    pub manifest: Manifest,
    pub sheet_urls: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowPayload {
    pub message: Message,
    pub quiet: bool,
    pub display_seconds: u32,
    pub pack: Option<PackView>,
}

/// Area of the popup (logical px, relative to the window) that should take
/// clicks. Everything else passes clicks through to the apps behind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.width && y <= self.y + self.height
    }
}
