//! go-ai-native-tcg-bridge — the LSP client seam over the CONSUMER's
//! own gopls (TCG-ORACLE-GO v0.1): resolution, framing, positions, the
//! correlated client, and the oracle's op surface.
//!
//! The unit suite is gopls-free (replay over scripted transports); the
//! live end-to-end test requires the tool and FAILS with the recipe
//! when it is absent — installing the stack obliges the machine to
//! carry gopls (ORACLE-GO §1).

specmark::scope!("spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#root");

use std::path::{Path, PathBuf};

use specmark::spec;

pub mod client;
pub mod frame;
pub mod oracle;
pub mod position;

pub use client::{Capabilities, LspClient, Transport};
pub use oracle::{Diagnostic, GoOracle, ValidateOutcome};

/// The environment override for the gopls binary — PATH is the
/// default; a machine that keeps tools off PATH (this project's own
/// dev box keeps them at `C:/opt/gotools`) points here.
pub const GOPLS_ENV_OVERRIDE: &str = "GO_AI_NATIVE_GOPLS";

/// The bridge's failure surface (TCG-PROTOCOL-GO §4): five kinds, each
/// a recipe, never a dead end; the two environment rows are the
/// deliberate renames against the sibling tables.
///
/// ```
/// use go_ai_native_tcg_bridge::TcgBridgeError;
/// let e = TcgBridgeError::GoplsMissing { detail: "not on PATH".into() };
/// assert!(e.to_string().contains("go install golang.org/x/tools/gopls@latest"));
/// assert_eq!(e.wire_kind(), "gopls-missing");
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-PROTOCOL-GO-v0.1#errors")]
pub enum TcgBridgeError {
    #[error(
        "violates spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#resolution: \
         no gopls resolvable ({detail}); fix surface: \
         `go install golang.org/x/tools/gopls@latest` (a stack prerequisite)"
    )]
    GoplsMissing { detail: String },

    #[error(
        "violates spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#session: \
         the workspace failed to load ({detail}); fix surface: run \
         `go env` and `go list ./...` in the project root and read the error"
    )]
    WorkspaceUnloadable { detail: String },

    #[error(
        "violates spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#lifecycle: \
         the gopls child is gone ({detail}); fix surface: the host registry \
         respawns once; run the op one-shot to see stderr"
    )]
    OracleCrashed { detail: String },

    #[error(
        "violates spec://go-ai-native-lang/go/mechanisms/TCG-PROTOCOL-GO-v0.1#parity: \
         protocol violation ({detail}); fix surface: rebuild the slot binary \
         so relay and host share one protocol"
    )]
    Protocol { detail: String },

    #[error(
        "violates spec://go-ai-native-lang/go/mechanisms/TCG-PROTOCOL-GO-v0.1#errors: \
         `{op}` did not answer within {budget_ms} ms; fix surface: raise the \
         caller's budget or check gopls health"
    )]
    Timeout { op: String, budget_ms: u64 },
}

impl TcgBridgeError {
    /// The TCG-PROTOCOL-GO §4 wire kind for this variant.
    ///
    /// ```
    /// use go_ai_native_tcg_bridge::TcgBridgeError;
    /// let e = TcgBridgeError::Timeout { op: "validate".into(), budget_ms: 5 };
    /// assert_eq!(e.wire_kind(), "timeout");
    /// ```
    pub fn wire_kind(&self) -> &'static str {
        match self {
            Self::GoplsMissing { .. } => "gopls-missing",
            Self::WorkspaceUnloadable { .. } => "workspace-unloadable",
            Self::OracleCrashed { .. } => "oracle-crashed",
            Self::Protocol { .. } => "protocol",
            Self::Timeout { .. } => "timeout",
        }
    }
}

/// The one config object the oracle ships (ORACLE-GO §3): gopls's
/// defaults are production-grade, so v0.1 is deliberately MINIMAL —
/// staticcheck integration stays off (the floor runs staticcheck
/// itself; one tool, one truth). Passed as `initializationOptions`
/// AND as every `workspace/configuration` answer.
///
/// ```
/// let c = go_ai_native_tcg_bridge::gopls_config();
/// assert!(c.is_object());
/// ```
#[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#config")]
pub fn gopls_config() -> serde_json::Value {
    serde_json::json!({})
}

/// Strip the Windows `\\?\` verbatim prefix — verbatim paths break
/// child argv and URI builders (the standing lesson's fifth home).
///
/// ```
/// use std::path::Path;
/// use go_ai_native_tcg_bridge::verbatim_free;
/// assert_eq!(
///     verbatim_free(Path::new(r"\\?\C:\x\y")),
///     std::path::PathBuf::from(r"C:\x\y"),
/// );
/// assert_eq!(verbatim_free(Path::new("/plain")), std::path::PathBuf::from("/plain"));
/// ```
pub fn verbatim_free(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(stripped) => PathBuf::from(stripped),
        None => path.to_path_buf(),
    }
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Resolve the CONSUMER's gopls (ORACLE-GO §1): the env override, then
/// PATH, then `$GOBIN`, then `$(go env GOPATH)/bin`, then the
/// recipe-carrying refusal. Returns the spawnable program (an absolute
/// path, or the bare name when PATH resolution is the winner).
#[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#resolution")]
pub fn resolve_gopls(root: &Path) -> Result<PathBuf, TcgBridgeError> {
    if let Ok(overridden) = std::env::var(GOPLS_ENV_OVERRIDE) {
        let p = PathBuf::from(&overridden);
        if p.is_file() {
            return Ok(verbatim_free(&p));
        }
        return Err(TcgBridgeError::GoplsMissing {
            detail: format!("{GOPLS_ENV_OVERRIDE}={overridden} is not a file"),
        });
    }
    // PATH: probe the bare name.
    let probe = std::process::Command::new("gopls")
        .arg("version")
        .current_dir(root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if let Ok(status) = probe
        && status.success()
    {
        return Ok(PathBuf::from("gopls"));
    }
    // GOBIN, then GOPATH/bin — resolved through the go binary the
    // extract bridge already locates.
    let go = go_ai_native_extract_bridge::go_binary();
    for var in ["GOBIN", "GOPATH"] {
        let out = std::process::Command::new(&go)
            .args(["env", var])
            .current_dir(root)
            .output();
        if let Ok(out) = out
            && out.status.success()
        {
            let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if value.is_empty() {
                continue;
            }
            let candidate = if var == "GOBIN" {
                PathBuf::from(&value).join(exe("gopls"))
            } else {
                PathBuf::from(&value).join("bin").join(exe("gopls"))
            };
            if candidate.is_file() {
                return Ok(verbatim_free(&candidate));
            }
        }
    }
    Err(TcgBridgeError::GoplsMissing {
        detail: format!(
            "neither {GOPLS_ENV_OVERRIDE}, PATH, GOBIN, nor GOPATH/bin resolves gopls \
             (probed from {})",
            root.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_carries_kind_and_recipe() {
        let cases: [(TcgBridgeError, &str, &str); 5] = [
            (
                TcgBridgeError::GoplsMissing { detail: "x".into() },
                "gopls-missing",
                "go install golang.org/x/tools/gopls@latest",
            ),
            (
                TcgBridgeError::WorkspaceUnloadable { detail: "x".into() },
                "workspace-unloadable",
                "go list",
            ),
            (
                TcgBridgeError::OracleCrashed { detail: "x".into() },
                "oracle-crashed",
                "respawns once",
            ),
            (
                TcgBridgeError::Protocol { detail: "x".into() },
                "protocol",
                "one protocol",
            ),
            (
                TcgBridgeError::Timeout {
                    op: "scope".into(),
                    budget_ms: 9,
                },
                "timeout",
                "budget",
            ),
        ];
        for (e, kind, hint) in cases {
            assert_eq!(e.wire_kind(), kind);
            let msg = e.to_string();
            assert!(
                msg.contains("spec://go-ai-native-lang/go/mechanisms/"),
                "{msg}"
            );
            assert!(msg.contains(hint), "{msg}");
        }
    }

    #[test]
    fn verbatim_free_is_identity_off_windows_prefixes() {
        assert_eq!(verbatim_free(Path::new("C:/a/b")), PathBuf::from("C:/a/b"));
    }
}
