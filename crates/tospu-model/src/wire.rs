use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::content::{Message, MessageItem};
use crate::manifest::Manifest;
use crate::settings::Settings;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackSummary {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<PackView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub settings: Settings,
    pub messages: Vec<MessageItem>,
    pub packs: Vec<PackSummary>,
    #[serde(default)]
    pub monitors: Vec<MonitorInfo>,
    pub problems: Vec<String>,
    pub launch_at_login: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_popup: Option<String>,
    pub shortcut_label: String,
    pub version: String,
    pub content_writable: bool,
    #[serde(default = "default_true")]
    pub settings_writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingInput {
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub gratitude: Option<String>,
    #[serde(default)]
    pub proud: Option<String>,
    pub active_pack: String,
    pub launch_at_login: bool,
}
