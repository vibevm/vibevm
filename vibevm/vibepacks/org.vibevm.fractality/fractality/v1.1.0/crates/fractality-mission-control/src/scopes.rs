//! Filesystem scopes and the rendezvous beacon (plan D19).
//!
//! v0.1 has exactly one scope: the runs root. Its id is minted once and
//! persisted in `<home>/scopes.toml`; the beacon at
//! `<runs-root>/.fractality-fsid` carries a fresh nonce every daemon
//! start (that *is* the v0.1 rotation policy — periodic-timer rotation
//! joins the federation work, DEF-6). References carry the scope id, not
//! the nonce, so rotation never invalidates a ref — it only re-proves
//! liveness.

use camino::Utf8Path;
use fractality_core::ids::ScopeId;
use fractality_core::node::{ScopeBeacon, ScopeInfo};
use fractality_core::time::now_ms;
use serde::{Deserialize, Serialize};

specmark::scope!("spec://fractality/PROP-001#invariants");

/// On-disk `scopes.toml`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
struct ScopesFile {
    #[serde(default)]
    scope: Vec<ScopeEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ScopeEntry {
    id: ScopeId,
    /// Well-known scope name; v0.1 knows exactly `runs`.
    name: String,
    created_ts_ms: u64,
}

const SCOPES_FILE: &str = "scopes.toml";
const RUNS_SCOPE_NAME: &str = "runs";

/// Loads-or-mints the runs scope id and stamps a fresh beacon at the
/// runs root. Returns the advertised scope set for `GET /v0/node`.
pub fn ensure_runs_scope(
    home: &Utf8Path,
    runs_root: &Utf8Path,
    mc_id: &str,
) -> Result<Vec<ScopeInfo>, String> {
    let path = home.join(SCOPES_FILE);
    let mut file: ScopesFile = match std::fs::read_to_string(path.as_std_path()) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("parsing `{path}`: {e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ScopesFile::default(),
        Err(e) => return Err(format!("reading `{path}`: {e}")),
    };

    let scope_id = match file.scope.iter().find(|s| s.name == RUNS_SCOPE_NAME) {
        Some(entry) => entry.id.clone(),
        None => {
            let id = ScopeId::new(ulid::Ulid::new().to_string());
            file.scope.push(ScopeEntry {
                id: id.clone(),
                name: RUNS_SCOPE_NAME.to_owned(),
                created_ts_ms: now_ms(),
            });
            let text =
                toml::to_string_pretty(&file).map_err(|e| format!("rendering scopes: {e}"))?;
            std::fs::write(path.as_std_path(), text)
                .map_err(|e| format!("writing `{path}`: {e}"))?;
            id
        }
    };

    std::fs::create_dir_all(runs_root.as_std_path())
        .map_err(|e| format!("creating `{runs_root}`: {e}"))?;
    let beacon = ScopeBeacon {
        schema: 1,
        scope_id: scope_id.clone(),
        nonce: ulid::Ulid::new().to_string(),
        mc_id: mc_id.to_owned(),
        issued_ts_ms: now_ms(),
    };
    let beacon_path = runs_root.join(ScopeBeacon::FILE_NAME);
    std::fs::write(
        beacon_path.as_std_path(),
        beacon
            .to_toml_string()
            .map_err(|e| format!("rendering beacon: {e}"))?,
    )
    .map_err(|e| format!("writing `{beacon_path}`: {e}"))?;

    Ok(vec![ScopeInfo {
        id: scope_id,
        root: runs_root.to_owned(),
        nonce: beacon.nonce,
        issued_ts_ms: beacon.issued_ts_ms,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn scratch(tag: &str) -> Utf8PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fractality-scope-{tag}-{}", ulid::Ulid::new()));
        Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
    }

    #[test]
    fn scope_id_is_stable_across_restarts_and_nonce_rotates() {
        let home = scratch("stable");
        std::fs::create_dir_all(home.as_std_path()).expect("mkdir");
        let runs = home.join("runs");

        let first = ensure_runs_scope(&home, &runs, "mc-a").expect("first stamp");
        let second = ensure_runs_scope(&home, &runs, "mc-b").expect("second stamp");
        assert_eq!(first[0].id, second[0].id, "scope id persists");
        assert_ne!(first[0].nonce, second[0].nonce, "nonce rotates per start");

        let beacon_text = std::fs::read_to_string(runs.join(ScopeBeacon::FILE_NAME).as_std_path())
            .expect("beacon exists");
        let beacon = ScopeBeacon::from_toml_str(&beacon_text).expect("beacon parses");
        assert_eq!(beacon.scope_id, second[0].id);
        assert_eq!(beacon.nonce, second[0].nonce);
        assert_eq!(beacon.mc_id, "mc-b");

        std::fs::remove_dir_all(home.as_std_path()).ok();
    }
}
