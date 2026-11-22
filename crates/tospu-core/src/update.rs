//! Pure semver comparison and GitHub Releases JSON parsing for manual update checks.

use serde::Deserialize;
use tospu_model::UpdateInfo;

pub const RELEASES_API_URL: &str = "https://api.github.com/repos/worty76/tospu/releases/latest";
pub const RELEASES_PAGE_URL: &str = "https://github.com/worty76/tospu/releases/latest";
pub const ALLOWED_RELEASE_PREFIX: &str = "https://github.com/worty76/tospu/";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    body: Option<String>,
}

/// Parses `"v0.2.1"`, `"0.2.1"`, or `"0.2.1-beta.1"` into `(0, 2, 1)`.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let clean = s.trim().trim_start_matches(['v', 'V']);
    let core = clean.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(cur), Some(lat)) => lat > cur,
        _ => false,
    }
}

pub fn parse_github_release(json: &str, current_version: &str) -> Result<UpdateInfo, String> {
    let rel: GithubRelease =
        serde_json::from_str(json).map_err(|e| format!("invalid release response: {e}"))?;
    let latest_version = rel
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();
    let release_url = rel
        .html_url
        .filter(|u| u.starts_with(ALLOWED_RELEASE_PREFIX))
        .unwrap_or_else(|| RELEASES_PAGE_URL.to_string());
    let notes = rel
        .body
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .map(|b| {
            let truncated: String = b.chars().take(240).collect();
            if b.chars().count() > 240 {
                format!("{truncated}…")
            } else {
                truncated
            }
        });
    let update_available = is_newer(current_version, &latest_version);

    Ok(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version,
        update_available,
        release_url,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_with_or_without_v_prefix() {
        assert!(is_newer("0.1.0", "v0.2.0"));
        assert!(is_newer("0.1.9", "0.1.10"));
        assert!(is_newer("0.9.9", "v1.0.0"));
        assert!(!is_newer("0.2.0", "v0.2.0"));
        assert!(!is_newer("0.2.0", "v0.1.9"));
        assert!(!is_newer("0.1.0", "not-a-version"));
    }

    #[test]
    fn parses_github_release_json() {
        let json = r#"{
            "tag_name": "v0.2.0",
            "html_url": "https://github.com/worty76/tospu/releases/tag/v0.2.0",
            "body": "Added cute new characters!"
        }"#;
        let info = parse_github_release(json, "0.1.0").unwrap();
        assert_eq!(info.current_version, "0.1.0");
        assert_eq!(info.latest_version, "0.2.0");
        assert!(info.update_available);
        assert_eq!(
            info.release_url,
            "https://github.com/worty76/tospu/releases/tag/v0.2.0"
        );
        assert_eq!(info.notes.as_deref(), Some("Added cute new characters!"));
    }
}
