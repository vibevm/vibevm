//! rust-ai-native-tcg-bridge — the LSP client seam over the CONSUMER's
//! own rust-analyzer (TCG-ORACLE-RUST v0.1): resolution, framing,
//! positions, the correlated client, and the oracle's op surface.
//!
//! The unit suite is rust-analyzer-free (replay over scripted
//! transports); the live end-to-end test requires the component and
//! FAILS with the recipe when it is absent — installing the stack
//! obliges the machine to carry rust-analyzer (ORACLE-RUST §1).

specmark::scope!("spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#root");

use std::path::{Path, PathBuf};

use specmark::spec;

pub mod client;
pub mod frame;
pub mod oracle;
pub mod position;

pub use client::{Capabilities, LspClient, Transport};
pub use oracle::{Diagnostic, RustOracle, ValidateOutcome};

/// The bridge's failure surface (TCG-PROTOCOL-RUST §4): five kinds,
/// each a recipe, never a dead end; the two environment rows are the
/// deliberate renames against the TS table.
///
/// ```
/// use rust_ai_native_tcg_bridge::TcgBridgeError;
/// let e = TcgBridgeError::RustAnalyzerMissing {
///     detail: "not on PATH".into(),
/// };
/// assert!(e.to_string().contains("rustup component add rust-analyzer"));
/// assert_eq!(e.wire_kind(), "rust-analyzer-missing");
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#errors")]
pub enum TcgBridgeError {
    #[error(
        "violates spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#resolution: \
         no rust-analyzer resolvable ({detail}); fix surface: \
         `rustup component add rust-analyzer` (a stack prerequisite)"
    )]
    RustAnalyzerMissing { detail: String },

    #[error(
        "violates spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#session: \
         the workspace failed to load ({detail}); fix surface: run \
         `cargo metadata` in the project root and read its error"
    )]
    WorkspaceUnloadable { detail: String },

    #[error(
        "violates spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#lifecycle: \
         the rust-analyzer child is gone ({detail}); fix surface: the host \
         registry respawns once; run the op one-shot to see stderr"
    )]
    OracleCrashed { detail: String },

    #[error(
        "violates spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#parity: \
         protocol violation ({detail}); fix surface: rebuild the slot binary \
         so relay and host share one protocol"
    )]
    Protocol { detail: String },

    #[error(
        "violates spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#errors: \
         `{op}` did not answer within {budget_ms} ms; fix surface: raise the \
         caller's budget or check rust-analyzer health"
    )]
    Timeout { op: String, budget_ms: u64 },
}

impl TcgBridgeError {
    /// The TCG-PROTOCOL-RUST §4 wire kind for this variant.
    ///
    /// ```
    /// use rust_ai_native_tcg_bridge::TcgBridgeError;
    /// let e = TcgBridgeError::Timeout { op: "validate".into(), budget_ms: 5 };
    /// assert_eq!(e.wire_kind(), "timeout");
    /// ```
    pub fn wire_kind(&self) -> &'static str {
        match self {
            Self::RustAnalyzerMissing { .. } => "rust-analyzer-missing",
            Self::WorkspaceUnloadable { .. } => "workspace-unloadable",
            Self::OracleCrashed { .. } => "oracle-crashed",
            Self::Protocol { .. } => "protocol",
            Self::Timeout { .. } => "timeout",
        }
    }
}

/// The one config object the oracle ships (ORACLE-RUST §3): r-a's
/// experimental diagnostics deliberately ON — a null-config oracle is
/// nearly blind (the Phase-0 spike's central finding). Passed as
/// `initializationOptions` AND as every `workspace/configuration`
/// answer.
///
/// ```
/// let c = rust_ai_native_tcg_bridge::ra_config();
/// assert_eq!(c["diagnostics"]["experimental"]["enable"], true);
/// ```
#[spec(implements = "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#config")]
pub fn ra_config() -> serde_json::Value {
    serde_json::json!({
        "diagnostics": { "experimental": { "enable": true } },
    })
}

/// Strip the Windows `\\?\` verbatim prefix — verbatim paths break
/// child argv and URI builders (the standing lesson's fourth home).
///
/// ```
/// use std::path::Path;
/// use rust_ai_native_tcg_bridge::verbatim_free;
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

/// Resolve the CONSUMER's rust-analyzer (ORACLE-RUST §1): the
/// toolchain component first (`rustup which`, run from the project
/// root so `rust-toolchain.toml` pinning is honoured), then PATH,
/// then the recipe-carrying refusal. Returns the spawnable program
/// (an absolute path, or the bare name when PATH resolution is the
/// winner).
#[spec(implements = "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#resolution")]
pub fn resolve_rust_analyzer(root: &Path) -> Result<PathBuf, TcgBridgeError> {
    let rustup = std::process::Command::new("rustup")
        .args(["which", "rust-analyzer"])
        .current_dir(root)
        .output();
    if let Ok(out) = rustup
        && out.status.success()
    {
        let path = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
        if path.is_file() {
            return Ok(verbatim_free(&path));
        }
    }
    // PATH fallback: probe the bare name.
    let probe = std::process::Command::new("rust-analyzer")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if let Ok(status) = probe
        && status.success()
    {
        return Ok(PathBuf::from("rust-analyzer"));
    }
    Err(TcgBridgeError::RustAnalyzerMissing {
        detail: format!(
            "neither `rustup which rust-analyzer` (from {}) nor PATH resolves it",
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
                TcgBridgeError::RustAnalyzerMissing { detail: "x".into() },
                "rust-analyzer-missing",
                "rustup component add",
            ),
            (
                TcgBridgeError::WorkspaceUnloadable { detail: "x".into() },
                "workspace-unloadable",
                "cargo metadata",
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
                msg.contains("spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/"),
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
