//! Bearer-token authentication for mutating endpoints.
//!
//! Tokens are loaded from `<data-dir>/state/admin.tokens` (one
//! token per line; `#` lines and blank lines ignored). Read endpoints
//! accept missing/invalid tokens silently; write endpoints require a
//! valid token and return 401 otherwise. Tokens never appear in
//! logs; the Authorization header is treated as a [PROP-000 §20]-
//! discipline secret.

specmark::scope!("spec://vibevm/common/PROP-000#token-secrecy");

use std::collections::BTreeSet;
use std::path::Path;

use crate::error::{Error, Result};

/// The bearer-token authority a write request is checked against — the
/// server's swappable auth seam. `AppState` holds one as `Box<dyn
/// TokenStore>`, so the production [`FileTokenStore`] (tokens read from
/// `<data-dir>/state/admin.tokens`) and any test double share one
/// contract. A second production variant (an external auth service, a
/// rotating-secret store) would land as another impl behind this trait
/// without touching a handler.
///
/// ```
/// use vibe_index::server::{FileTokenStore, TokenStore};
///
/// // The production store with no `admin.tokens` file behind it holds
/// // no tokens, so it authorises no writes.
/// let store = FileTokenStore::default();
/// assert!(!store.has_any());
/// assert!(!store.check("anything"));
/// ```
pub trait TokenStore: std::fmt::Debug + Send + Sync {
    /// Returns `true` if at least one token is configured; `false` means
    /// the server permits no mutations regardless of header presence.
    fn has_any(&self) -> bool;

    /// Returns `true` iff `supplied` is an accepted admin token.
    fn check(&self, supplied: &str) -> bool;
}

/// File-backed [`TokenStore`] — the production variant, one token per
/// line of `<data-dir>/state/admin.tokens`.
#[derive(Debug, Default, Clone)]
pub struct FileTokenStore {
    tokens: BTreeSet<String>,
}

impl FileTokenStore {
    pub fn load(data_dir: &Path) -> Result<Self> {
        Self::load_from_path(&data_dir.join("state").join("admin.tokens"))
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FileTokenStore::default());
            }
            Err(e) => {
                return Err(Error::Io {
                    path: path.to_path_buf(),
                    message: e.to_string(),
                });
            }
        };
        let s = std::str::from_utf8(&bytes)
            .map_err(|e| Error::Malformed(format!("admin.tokens not UTF-8: {e}")))?;
        let tokens = s
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_string())
            .collect();
        Ok(FileTokenStore { tokens })
    }
}

impl TokenStore for FileTokenStore {
    /// Constant-time-ish check — iterates every token. Practical
    /// against the small file scale we target (<100 tokens).
    fn check(&self, supplied: &str) -> bool {
        self.tokens.iter().any(|t| t == supplied)
    }

    fn has_any(&self) -> bool {
        !self.tokens.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_empty_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileTokenStore::load(dir.path()).unwrap();
        assert!(!store.has_any());
        assert!(!store.check("anything"));
    }

    #[test]
    fn load_parses_one_token_per_line() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let path = state.join("admin.tokens");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# this line ignored").unwrap();
        writeln!(f, "alpha").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "  beta-gamma  ").unwrap();
        f.sync_all().unwrap();
        let store = FileTokenStore::load(dir.path()).unwrap();
        assert!(store.check("alpha"));
        assert!(store.check("beta-gamma"));
        assert!(!store.check("comment"));
        assert!(!store.check("# this line ignored"));
    }
}
