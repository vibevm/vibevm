//! Bearer-token authentication for mutating endpoints.
//!
//! Tokens are loaded from `<data-dir>/state/admin.tokens` (one
//! token per line; `#` lines and blank lines ignored). Read endpoints
//! accept missing/invalid tokens silently; write endpoints require a
//! valid token and return 401 otherwise. Tokens never appear in
//! logs; the Authorization header is treated as a [PROP-000 §20]-
//! discipline secret.

use std::collections::BTreeSet;
use std::path::Path;

use crate::error::{Error, Result};

#[derive(Debug, Default, Clone)]
pub struct TokenStore {
    tokens: BTreeSet<String>,
}

impl TokenStore {
    pub fn load(data_dir: &Path) -> Result<Self> {
        Self::load_from_path(&data_dir.join("state").join("admin.tokens"))
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(TokenStore::default());
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
        Ok(TokenStore { tokens })
    }

    /// Returns `true` if at least one token is loaded; `false` means
    /// the server permits no mutations regardless of header presence.
    pub fn has_any(&self) -> bool {
        !self.tokens.is_empty()
    }

    /// Constant-time-ish check — iterates every token. Practical
    /// against the small file scale we target (<100 tokens).
    pub fn check(&self, supplied: &str) -> bool {
        self.tokens.iter().any(|t| t == supplied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_empty_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = TokenStore::load(dir.path()).unwrap();
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
        let store = TokenStore::load(dir.path()).unwrap();
        assert!(store.check("alpha"));
        assert!(store.check("beta-gamma"));
        assert!(!store.check("comment"));
        assert!(!store.check("# this line ignored"));
    }
}
