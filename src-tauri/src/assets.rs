//! Converts local filesystem paths into Tauri asset protocol URLs.

use std::path::Path;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

/// Matches JavaScript `encodeURIComponent` unescaped characters:
/// `A-Z a-z 0-9 - _ . ! ~ * ' ( )`.
const URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// Drops the `\\?\` verbatim prefix Windows sometimes puts on resolved
/// paths; the asset protocol scope only matches plain `C:\...` paths.
fn strip_verbatim_prefix(path: &str) -> &str {
    match path.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => rest,
        _ => path,
    }
}

pub fn asset_url(path: &Path) -> String {
    let path = path.to_string_lossy();
    let encoded = utf8_percent_encode(strip_verbatim_prefix(&path), URI_COMPONENT).to_string();
    #[cfg(windows)]
    {
        format!("http://asset.localhost/{encoded}")
    }
    #[cfg(not(windows))]
    {
        format!("asset://localhost/{encoded}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_windows_verbatim_prefix() {
        assert_eq!(
            strip_verbatim_prefix(r"\\?\C:\Program Files\Tospu\a.png"),
            r"C:\Program Files\Tospu\a.png"
        );
        assert_eq!(strip_verbatim_prefix(r"C:\a.png"), r"C:\a.png");
        assert_eq!(
            strip_verbatim_prefix(r"\\?\UNC\srv\a.png"),
            r"\\?\UNC\srv\a.png"
        );
        assert_eq!(strip_verbatim_prefix("/a/b.png"), "/a/b.png");
    }

    #[cfg(not(windows))]
    #[test]
    fn encodes_like_encode_uri_component() {
        assert_eq!(
            asset_url(Path::new("/a b/ộ.png")),
            "asset://localhost/%2Fa%20b%2F%E1%BB%99.png"
        );
    }
}
