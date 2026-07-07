//! The per-language oracle registry (PROP-026 §4): lazy resolve →
//! consent-checked build → spawn `tcg-typescript serve` → hold the
//! handle across calls → one transparent respawn → kill on drop.
//!
//! The child link is a seam (`OracleLink` + a `Spawner`) so every layer
//! above process mechanics is tested with a double — no node, no cargo,
//! no slot anywhere near the unit suite.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-026#registry");

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

use vibe_workspace::bins::{BinsError, build_binary, collect_binaries, find_binary};

use crate::{TcgError, TcgHost};

/// The wire protocol the relay speaks (TCG-PROTOCOL v0.1).
const ORACLE_PROTOCOL: u64 = 1;
/// Cold init compiles a real program; everything after is milliseconds.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// One live relay link: send a frame, receive the matching frame.
///
/// ```
/// use vibe_tcg::{OracleLink, TcgError};
/// struct Echo;
/// impl OracleLink for Echo {
///     fn request(
///         &mut self,
///         frame: serde_json::Value,
///     ) -> Result<serde_json::Value, TcgError> {
///         Ok(serde_json::json!({ "echo": frame["op"] }))
///     }
/// }
/// let mut link = Echo;
/// let out = link.request(serde_json::json!({"op": "validate"})).unwrap();
/// assert_eq!(out["echo"], "validate");
/// ```
pub trait OracleLink: Send {
    fn request(&mut self, frame: serde_json::Value) -> Result<serde_json::Value, TcgError>;
}

/// Spawns a link for (language, artifact, project_root). Production
/// spawns the slot binary; tests inject doubles. The language rides
/// along so every error the link raises names ITS fix surface.
pub type Spawner =
    Box<dyn Fn(&str, &Path, &Path) -> Result<Box<dyn OracleLink>, TcgError> + Send + Sync>;

/// The per-language dispatch table (PROP-026 §4): the relay binary and
/// the requires-line its not-installed recipe names. A NEW language is
/// one row here plus one `LANGUAGES` entry — never new tools.
fn language_binary(language: &str) -> &'static str {
    match language {
        "typescript" => "tcg-typescript",
        "rust" => "tcg-rust",
        _ => unreachable!("run_tool validates the language first"),
    }
}

fn language_requires(language: &str) -> &'static str {
    match language {
        "typescript" => "\"stack:org.vibevm/typescript-ai-native\" = \"^0.4\"",
        "rust" => "\"stack:org.vibevm/rust-ai-native\" = \"^0.5\"",
        _ => unreachable!("run_tool validates the language first"),
    }
}

/// The registry: interior-mutable (the tool seam hands out shared
/// refs), lazily populated, children die with it.
pub struct OracleRegistry {
    children: Mutex<HashMap<String, Box<dyn OracleLink>>>,
    spawner: Spawner,
}

impl Default for OracleRegistry {
    fn default() -> Self {
        Self::with_spawner(Box::new(|language, artifact, root| {
            Ok(Box::new(ProcessLink::spawn(language, artifact, root)?) as Box<dyn OracleLink>)
        }))
    }
}

impl OracleRegistry {
    pub fn with_spawner(spawner: Spawner) -> Self {
        Self {
            children: Mutex::new(HashMap::new()),
            spawner,
        }
    }

    /// Resolve the language's relay binary through the CURRENT
    /// project's lockfile (PROP-025 dispatch via the shared bins cell),
    /// applying the §5 no-prompt consent rule.
    fn resolve_artifact(
        &self,
        language: &str,
        root: &Path,
    ) -> Result<std::path::PathBuf, TcgError> {
        let binary = language_binary(language);
        let bins = collect_binaries(root).map_err(|e| TcgError::Protocol {
            detail: format!("resolving declared binaries: {e}"),
        })?;
        let bin = match find_binary(&bins, binary) {
            Ok(b) => b,
            Err(BinsError::UnknownBinary { .. }) => {
                return Err(TcgError::StackNotInstalled {
                    language: language.to_string(),
                    binary: binary.to_string(),
                    requires: language_requires(language),
                });
            }
            Err(e) => {
                return Err(TcgError::Protocol {
                    detail: format!("binary resolution: {e}"),
                });
            }
        };
        if !bin.artifact().exists() {
            if bin.group == "org.vibevm" {
                build_binary(bin, false).map_err(|e| TcgError::BuildFailed {
                    binary: binary.to_string(),
                    detail: e.to_string(),
                })?;
            } else {
                return Err(TcgError::NotBuiltThirdParty {
                    binary: binary.to_string(),
                    package: bin.package.clone(),
                });
            }
        }
        Ok(bin.artifact())
    }

    fn ensure_and_send(
        &self,
        language: &str,
        host: &dyn TcgHost,
        frame: &serde_json::Value,
    ) -> Result<serde_json::Value, TcgError> {
        let mut children = self
            .children
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let link = match children.entry(language.to_string()) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                let artifact = self.resolve_artifact(language, host.project_root())?;
                v.insert((self.spawner)(language, &artifact, host.project_root())?)
            }
        };
        let outcome = link.request(frame.clone());
        if matches!(outcome, Err(TcgError::OracleGone { .. })) {
            children.remove(language);
        }
        outcome
    }

    /// One op against the language's relay, with the PROP-026 §4
    /// respawn-once policy on a dead child.
    pub fn request(
        &self,
        language: &str,
        host: &dyn TcgHost,
        op: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TcgError> {
        let frame = serde_json::json!({
            "proto": ORACLE_PROTOCOL,
            // ids are per-link (the link owns correlation); the registry
            // frame carries a placeholder the link rewrites.
            "id": 0,
            "op": op,
            "params": params,
        });
        match self.ensure_and_send(language, host, &frame) {
            Err(TcgError::OracleGone { .. }) => {
                // one transparent respawn, then surface
                self.ensure_and_send(language, host, &frame)
                    .map_err(|e| match e {
                        TcgError::OracleGone {
                            language,
                            binary,
                            detail,
                        } => TcgError::OracleGone {
                            language,
                            binary,
                            detail: format!("{detail} (after one respawn)"),
                        },
                        other => other,
                    })
            }
            outcome => outcome,
        }
    }
}

/// The production link: the relay process with piped stdio and a
/// reader thread (the same mechanics the package bridges proved).
/// Carries ITS language and binary so no error ever names another
/// language's fix surface.
struct ProcessLink {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: Receiver<Result<String, std::io::Error>>,
    next_id: u64,
    language: String,
    binary: &'static str,
    root: String,
}

impl ProcessLink {
    fn spawn(language: &str, artifact: &Path, root: &Path) -> Result<Self, TcgError> {
        let binary = language_binary(language);
        let gone = |detail: String| TcgError::OracleGone {
            language: language.to_string(),
            binary,
            detail,
        };
        let mut child = std::process::Command::new(artifact)
            .arg("serve")
            .arg("--root")
            .arg(root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| gone(format!("spawning {}: {e}", artifact.display())))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| gone("relay stdin not piped".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| gone("relay stdout not piped".to_string()))?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            lines: rx,
            next_id: 1,
            language: language.to_string(),
            binary,
            root: root.to_string_lossy().into_owned(),
        })
    }

    fn gone(&self, detail: String) -> TcgError {
        TcgError::OracleGone {
            language: format!("{} ({})", self.language, self.root),
            binary: self.binary,
            detail,
        }
    }
}

impl Drop for ProcessLink {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl OracleLink for ProcessLink {
    fn request(&mut self, mut frame: serde_json::Value) -> Result<serde_json::Value, TcgError> {
        let id = self.next_id;
        self.next_id += 1;
        frame["id"] = serde_json::json!(id);
        writeln!(self.stdin, "{frame}")
            .and_then(|()| self.stdin.flush())
            .map_err(|e| self.gone(format!("writing request: {e}")))?;
        let deadline = std::time::Instant::now() + REQUEST_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(TcgError::Protocol {
                    detail: format!("relay did not answer within {REQUEST_TIMEOUT:?}"),
                });
            }
            let raw = match self.lines.recv_timeout(remaining) {
                Ok(Ok(l)) => l,
                Ok(Err(e)) => return Err(self.gone(format!("reading response: {e}"))),
                Err(RecvTimeoutError::Timeout) => {
                    return Err(TcgError::Protocol {
                        detail: format!("relay did not answer within {REQUEST_TIMEOUT:?}"),
                    });
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(self.gone("relay stream closed".to_string()));
                }
            };
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(trimmed).map_err(|e| TcgError::Protocol {
                    detail: format!("unparseable relay frame: {e}"),
                })?;
            if value.get("id").and_then(|v| v.as_u64()) != Some(id) {
                continue; // stale frame from a previous request; skip
            }
            return if value.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                Ok(value
                    .get("result")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null))
            } else {
                let detail = value
                    .get("error")
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "ok:false with no error body".to_string());
                let kind = value
                    .pointer("/error/kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("protocol");
                Err(match kind {
                    "oracle-crashed" => self.gone(detail),
                    _ => TcgError::Protocol { detail },
                })
            };
        }
    }
}

#[cfg(test)]
#[path = "registry/tests.rs"]
mod tests;
