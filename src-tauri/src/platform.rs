//! Process-level tweaks that must run before Tauri starts.

/// Environment variables to set on Linux, given a lookup for the current ones.
///
/// - Under Wayland the popup can't place itself, read the cursor or receive
///   the global shortcut, so run through XWayland unless the user opts out
///   with `TOSPU_WAYLAND=1` or picked a GDK backend themselves.
/// - WebKitGTK's DMA-BUF renderer shows blank windows on some GPU drivers and
///   VMs; turn it off unless the user already chose.
pub fn linux_env_fixes(get: impl Fn(&str) -> Option<String>) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    let on_wayland = get("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    if on_wayland && get("GDK_BACKEND").is_none() && get("TOSPU_WAYLAND").as_deref() != Some("1") {
        out.push(("GDK_BACKEND", "x11"));
    }
    if get("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        out.push(("WEBKIT_DISABLE_DMABUF_RENDERER", "1"));
    }
    out
}

/// Apply [`linux_env_fixes`] to this process. Call first thing in `main`,
/// before any threads exist.
pub fn prepare_environment() {
    #[cfg(target_os = "linux")]
    for (key, value) in linux_env_fixes(|k| std::env::var(k).ok()) {
        std::env::set_var(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fixes(vars: &[(&str, &str)]) -> Vec<(&'static str, &'static str)> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        linux_env_fixes(|k| map.get(k).cloned())
    }

    #[test]
    fn wayland_falls_back_to_x11() {
        assert!(fixes(&[("WAYLAND_DISPLAY", "wayland-0")]).contains(&("GDK_BACKEND", "x11")));
    }

    #[test]
    fn x11_session_keeps_backend() {
        assert!(!fixes(&[("DISPLAY", ":0")])
            .iter()
            .any(|(k, _)| *k == "GDK_BACKEND"));
    }

    #[test]
    fn user_choices_win() {
        let f = fixes(&[
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("TOSPU_WAYLAND", "1"),
            ("WEBKIT_DISABLE_DMABUF_RENDERER", "0"),
        ]);
        assert!(f.is_empty());
        let f = fixes(&[("WAYLAND_DISPLAY", "wayland-0"), ("GDK_BACKEND", "wayland")]);
        assert!(!f.iter().any(|(k, _)| *k == "GDK_BACKEND"));
    }

    #[test]
    fn disables_dmabuf_by_default() {
        assert!(fixes(&[]).contains(&("WEBKIT_DISABLE_DMABUF_RENDERER", "1")));
    }
}
