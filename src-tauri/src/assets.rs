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

pub fn asset_url(path: &Path) -> String {
    let encoded = utf8_percent_encode(&path.to_string_lossy(), URI_COMPONENT).to_string();
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

    #[cfg(not(windows))]
    #[test]
    fn encodes_like_encode_uri_component() {
        assert_eq!(
            asset_url(Path::new("/a b/ộ.png")),
            "asset://localhost/%2Fa%20b%2F%E1%BB%99.png"
        );
    }
}
