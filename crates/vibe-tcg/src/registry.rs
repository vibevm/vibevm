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

/// Spawns a link for (artifact, project_root). Production spawns the
/// slot binary; tests inject doubles.
pub type Spawner = Box<dyn Fn(&Path, &Path) -> Result<Box<dyn OracleLink>, TcgError> + Send + Sync>;

fn language_binary(language: &str) -> &'static str {
    match language {
        "typescript" => "tcg-typescript",
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
        Self::with_spawner(Box::new(|artifact, root| {
            Ok(Box::new(ProcessLink::spawn(artifact, root)?) as Box<dyn OracleLink>)
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
                v.insert((self.spawner)(&artifact, host.project_root())?)
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
                        TcgError::OracleGone { language, detail } => TcgError::OracleGone {
                            language,
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
/// reader thread (the same mechanics the package bridge proved).
struct ProcessLink {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: Receiver<Result<String, std::io::Error>>,
    next_id: u64,
    language_root: String,
}

impl ProcessLink {
    fn spawn(artifact: &Path, root: &Path) -> Result<Self, TcgError> {
        let mut child = std::process::Command::new(artifact)
            .arg("serve")
            .arg("--root")
            .arg(root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| TcgError::OracleGone {
                language: "typescript".to_string(),
                detail: format!("spawning {}: {e}", artifact.display()),
            })?;
        let stdin = child.stdin.take().ok_or_else(|| TcgError::OracleGone {
            language: "typescript".to_string(),
            detail: "relay stdin not piped".to_string(),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| TcgError::OracleGone {
            language: "typescript".to_string(),
            detail: "relay stdout not piped".to_string(),
        })?;
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
            language_root: root.to_string_lossy().into_owned(),
        })
    }

    fn gone(&self, detail: String) -> TcgError {
        TcgError::OracleGone {
            language: format!("typescript ({})", self.language_root),
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
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct H(std::path::PathBuf);
    impl TcgHost for H {
        fn project_root(&self) -> &Path {
            &self.0
        }
    }

    /// A scripted link: answers, or dies once.
    struct DoubleLink {
        die_first: bool,
        calls: Arc<AtomicUsize>,
    }
    impl OracleLink for DoubleLink {
        fn request(&mut self, frame: serde_json::Value) -> Result<serde_json::Value, TcgError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.die_first {
                self.die_first = false;
                return Err(TcgError::OracleGone {
                    language: "typescript".to_string(),
                    detail: "scripted death".to_string(),
                });
            }
            Ok(serde_json::json!({
                "echo_op": frame["op"],
                "echo_file": frame["params"]["file"],
            }))
        }
    }

    fn fixture_project_with_artifact() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .expect("vibe.toml");
        std::fs::write(
            dir.path().join("vibe.lock"),
            r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-07-07T00:00:00Z"
schema_version = 5

[[package]]
kind = "stack"
group = "org.vibevm"
name = "typescript-ai-native"
version = "0.4.0"
registry = "vibespecs"
source_url = "file://packages"
source_ref = "v0.4.0"
content_hash = "sha256:deadbeef"
files_written = []
"#,
        )
        .expect("vibe.lock");
        let slot = dir
            .path()
            .join("vibedeps")
            .join("stack-typescript-ai-native")
            .join("0.4.0");
        let release = slot.join("target").join("release");
        std::fs::create_dir_all(&release).expect("release dir");
        std::fs::write(
            slot.join("vibe.toml"),
            r#"[package]
name = "typescript-ai-native"
group = "org.vibevm"
kind = "stack"
version = "0.4.0"
authors = ["x"]
license = "EULA"
description = "fixture"
keywords = []

[[binary]]
name = "tcg-typescript"
crate = "crates/tcg-cli-typescript"
"#,
        )
        .expect("slot manifest");
        // a pre-"built" artifact so resolve_artifact never runs cargo
        let artifact = release.join(if cfg!(windows) {
            "tcg-typescript.exe"
        } else {
            "tcg-typescript"
        });
        std::fs::write(&artifact, b"fake").expect("artifact");
        dir
    }

    #[test]
    fn requests_relay_through_the_spawned_link() {
        let dir = fixture_project_with_artifact();
        let host = H(dir.path().to_path_buf());
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in = calls.clone();
        let registry = OracleRegistry::with_spawner(Box::new(move |_a, _r| {
            Ok(Box::new(DoubleLink {
                die_first: false,
                calls: calls_in.clone(),
            }) as Box<dyn OracleLink>)
        }));
        let out = registry
            .request(
                "typescript",
                &host,
                "validate",
                serde_json::json!({"file": "src/a.ts"}),
            )
            .expect("relayed");
        assert_eq!(out["echo_op"], "validate");
        assert_eq!(out["echo_file"], "src/a.ts");
        // second call reuses the SAME link (lazy, persistent)
        let _ = registry
            .request(
                "typescript",
                &host,
                "scope",
                serde_json::json!({"file": "src/a.ts"}),
            )
            .expect("relayed again");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_dead_link_is_respawned_exactly_once() {
        let dir = fixture_project_with_artifact();
        let host = H(dir.path().to_path_buf());
        let spawns = Arc::new(AtomicUsize::new(0));
        let spawns_in = spawns.clone();
        let registry = OracleRegistry::with_spawner(Box::new(move |_a, _r| {
            let n = spawns_in.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(DoubleLink {
                die_first: n == 0, // the first link dies on its first request
                calls: Arc::new(AtomicUsize::new(0)),
            }) as Box<dyn OracleLink>)
        }));
        let out = registry
            .request(
                "typescript",
                &host,
                "type",
                serde_json::json!({"file": "src/a.ts"}),
            )
            .expect("survived one death");
        assert_eq!(out["echo_op"], "type");
        assert_eq!(spawns.load(Ordering::SeqCst), 2, "exactly one respawn");
    }

    #[test]
    fn stack_absent_names_the_requires_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .expect("vibe.toml");
        let host = H(dir.path().to_path_buf());
        let registry = OracleRegistry::with_spawner(Box::new(|_a, _r| {
            panic!("must not spawn without a resolved artifact")
        }));
        let err = registry
            .request(
                "typescript",
                &host,
                "validate",
                serde_json::json!({"file": "src/a.ts"}),
            )
            .expect_err("not installed");
        assert!(matches!(err, TcgError::StackNotInstalled { .. }));
        assert!(err.to_string().contains("vibe install"), "{err}");
    }

    #[test]
    fn third_party_unbuilt_is_refused_with_the_recipe() {
        let dir = fixture_project_with_artifact();
        // make the package foreign and remove the artifact
        let slot = dir
            .path()
            .join("vibedeps")
            .join("stack-typescript-ai-native")
            .join("0.4.0");
        let manifest = std::fs::read_to_string(slot.join("vibe.toml")).expect("read");
        std::fs::write(
            slot.join("vibe.toml"),
            manifest.replace("group = \"org.vibevm\"", "group = \"com.example\""),
        )
        .expect("rewrite");
        std::fs::remove_file(slot.join("target").join("release").join(if cfg!(windows) {
            "tcg-typescript.exe"
        } else {
            "tcg-typescript"
        }))
        .expect("rm artifact");
        // the lockfile group must match the manifest group for the walk
        let lock = std::fs::read_to_string(dir.path().join("vibe.lock")).expect("lock");
        std::fs::write(
            dir.path().join("vibe.lock"),
            lock.replace("group = \"org.vibevm\"", "group = \"com.example\""),
        )
        .expect("rewrite lock");

        let host = H(dir.path().to_path_buf());
        let registry = OracleRegistry::with_spawner(Box::new(|_a, _r| {
            panic!("must not spawn an unbuilt third-party binary")
        }));
        let err = registry
            .request(
                "typescript",
                &host,
                "validate",
                serde_json::json!({"file": "src/a.ts"}),
            )
            .expect_err("refused");
        assert!(matches!(err, TcgError::NotBuiltThirdParty { .. }));
        assert!(err.to_string().contains("--assume-yes"), "{err}");
    }
}
