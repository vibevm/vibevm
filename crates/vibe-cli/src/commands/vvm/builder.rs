//! The build seam (PROP-019 §2.7): compile a source tree into the essential
//! `vibe` and `vibe-index` binaries. A crate-internal seam so tests drive
//! the pipeline without a real cargo build.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#build");

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::progress::{Progress, ProgressDiagnosticLevel, ProgressTask};

use super::model::{Profile, VersionId};

/// A selector resolved to a concrete version id and commit (PROP-019 §2.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedVersion {
    pub id: VersionId,
    pub commit: String,
}

/// The product of a successful build: where the binary landed and which
/// toolchain produced it (PROP-019 §2.7).
#[derive(Debug, Clone)]
pub(crate) struct BuildOutput {
    pub binary: PathBuf,
    pub index_binary: PathBuf,
    pub toolchain: String,
}

/// Builds a vibevm source tree into both essential binaries (PROP-019 §2.7).
pub(crate) trait Builder {
    fn build(&self, source_root: &Path, target_dir: &Path, profile: Profile)
    -> Result<BuildOutput>;
}

/// The production builder builds both essential packages into a
/// managed `--target-dir`, honouring the tree's `rust-toolchain.toml`
/// (PROP-019 §2.7, §2.8).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#build")]
pub(crate) struct CargoBuilder {
    /// The invocation's resolved offline posture, handed down from the
    /// composition root. A source rebuild is the one place the version
    /// manager pulls crates rather than bytes, so the posture has to reach
    /// cargo itself or an "offline" rebuild would still go to the network
    /// for a dependency (PROP-019 `##CMD-OFFLINE`).
    offline: bool,
    progress: Progress,
}

impl CargoBuilder {
    pub(crate) fn new(offline: bool, progress: Progress) -> Self {
        Self { offline, progress }
    }
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-060#SELF-STAGES")]
impl Builder for CargoBuilder {
    fn build(
        &self,
        source_root: &Path,
        target_dir: &Path,
        profile: Profile,
    ) -> Result<BuildOutput> {
        let build = self.progress.task("Building essential binaries");
        build.set_progress(0, Some(2), "components");
        let result = self.build_observed(source_root, target_dir, profile, &build);
        if result.is_ok() {
            build.finish();
        } else {
            build.fail("cargo build failed");
        }
        result
    }
}

impl CargoBuilder {
    fn build_observed(
        &self,
        source_root: &Path,
        target_dir: &Path,
        profile: Profile,
        build: &ProgressTask,
    ) -> Result<BuildOutput> {
        // Build into a VVM-managed `--target-dir`, never the source tree's
        // own `target/` — keeps the dev tree clean and, load-bearing on
        // Windows, avoids relinking a `vibe.exe` that is running (PROP-019
        // §2.7, §9.3).
        let mut cmd = Command::new("cargo");
        cmd.current_dir(source_root)
            .args(cargo_build_args(profile, self.offline));
        cmd.arg("--target-dir")
            .arg(target_dir)
            .stderr(Stdio::piped());
        build.detail(format!(
            "cargo build --locked -p vibe-cli -p vibe-index ({})",
            profile.target_subdir()
        ));
        let mut child = cmd
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawning cargo build in `{}`", source_root.display()))?;
        let stdout = child
            .stdout
            .take()
            .context("cargo build stdout was not piped")?;
        let stderr = child
            .stderr
            .take()
            .context("cargo build stderr was not piped")?;
        let components = build.progress();
        let vibe = components.task("Awaiting vibe artifact");
        let index = components.task("Awaiting vibe-index artifact");
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let mut details = ProgressDetailWriter {
            task: build,
            tail: Arc::clone(&tail),
        };
        let selected = std::thread::scope(|scope| {
            let stderr_tail = Arc::clone(&tail);
            let stderr_reader = scope.spawn(move || -> Result<()> {
                for line in BufReader::new(stderr).lines() {
                    emit_child_line(build, &stderr_tail, &line.context("reading cargo stderr")?);
                }
                Ok(())
            });
            let selected = select_artifacts(BufReader::new(stdout), &mut details);
            let stderr_result = stderr_reader
                .join()
                .map_err(|_| anyhow::anyhow!("cargo stderr reader panicked"))?;
            stderr_result?;
            selected
        });
        let status = child.wait().context("waiting for cargo build")?;
        if !status.success() {
            vibe.fail("component build did not complete");
            index.fail("component build did not complete");
            bail!(
                "cargo build ({}) failed (exit {:?}){}",
                profile.target_subdir(),
                status.code(),
                diagnostic_tail(&tail),
            );
        }
        let (binary, index_binary) = match selected {
            Ok(selected) => selected,
            Err(error) => {
                vibe.fail("vibe artifact selection failed");
                index.fail("vibe-index artifact selection failed");
                return Err(error);
            }
        };
        if !binary.is_file() || !index_binary.is_file() {
            vibe.fail("vibe artifact is missing");
            index.fail("vibe-index artifact is missing");
            bail!(
                "build reported success but essential binaries are missing: `{}`, `{}`",
                binary.display(),
                index_binary.display()
            );
        }
        vibe.finish();
        build.set_progress(1, Some(2), "components");
        index.finish();
        build.set_progress(2, Some(2), "components");
        let toolchain = Command::new("rustc")
            .current_dir(source_root)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Ok(BuildOutput {
            binary,
            index_binary,
            toolchain,
        })
    }
}

struct ProgressDetailWriter<'a> {
    task: &'a ProgressTask,
    tail: Arc<Mutex<VecDeque<String>>>,
}

impl Write for ProgressDetailWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        emit_child_line(self.task, &self.tail, &String::from_utf8_lossy(bytes));
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn emit_child_line(task: &ProgressTask, tail: &Mutex<VecDeque<String>>, text: &str) {
    for line in text
        .lines()
        .map(sanitize_child_line)
        .filter(|line| !line.is_empty())
    {
        let trimmed = line.trim_start();
        if trimmed.starts_with("error:") || trimmed.starts_with("error[") {
            task.diagnostic(ProgressDiagnosticLevel::Error, line.clone());
        } else if trimmed.starts_with("warning:") || trimmed.starts_with("warning[") {
            task.diagnostic(ProgressDiagnosticLevel::Warning, line.clone());
        } else {
            task.detail(line.clone());
        }
        if let Ok(mut tail) = tail.lock() {
            tail.push_back(line);
            while tail.len() > 16 {
                tail.pop_front();
            }
        }
    }
}

fn sanitize_child_line(text: &str) -> String {
    crate::output::sanitize_progress_text(text)
}

fn diagnostic_tail(tail: &Mutex<VecDeque<String>>) -> String {
    let Ok(tail) = tail.lock() else {
        return String::new();
    };
    if tail.is_empty() {
        String::new()
    } else {
        format!(
            "\nCargo diagnostics:\n{}",
            tail.iter().cloned().collect::<Vec<_>>().join("\n")
        )
    }
}

fn cargo_build_args(profile: Profile, offline: bool) -> Vec<&'static str> {
    let mut args = vec![
        "build",
        "--locked",
        "-p",
        "vibe-cli",
        "-p",
        "vibe-index",
        "--message-format=json-render-diagnostics",
    ];
    if profile == Profile::Release {
        args.push("--release");
    }
    if offline {
        // Cargo's own word for the same posture: resolve and build from the
        // machine's existing registry cache and vendor directory, and fail
        // — naming the crate — rather than reach for one it lacks.
        args.push("--offline");
    }
    args
}

fn select_artifacts(
    reader: impl BufRead,
    diagnostics: &mut impl Write,
) -> Result<(PathBuf, PathBuf)> {
    let mut vibe = Vec::new();
    let mut index = Vec::new();
    for line in reader.lines() {
        let line = line.context("reading Cargo JSON message")?;
        let value: serde_json::Value =
            serde_json::from_str(&line).context("decoding Cargo JSON message")?;
        match value.get("reason").and_then(|value| value.as_str()) {
            Some("compiler-message") => {
                if let Some(rendered) = value
                    .get("message")
                    .and_then(|message| message.get("rendered"))
                    .and_then(|rendered| rendered.as_str())
                {
                    diagnostics.write_all(rendered.as_bytes())?;
                }
            }
            Some("compiler-artifact") => {
                let target = &value["target"];
                let is_bin = target["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"));
                if !is_bin {
                    continue;
                }
                let name = target["name"].as_str().unwrap_or_default();
                if !matches!(name, "vibe" | "vibe-index") {
                    continue;
                }
                let executable = value["executable"]
                    .as_str()
                    .with_context(|| format!("Cargo artifact `{name}` omitted executable"))?;
                match name {
                    "vibe" => {
                        vibe.push(PathBuf::from(executable));
                    }
                    "vibe-index" => {
                        index.push(PathBuf::from(executable));
                    }
                    _ => unreachable!(),
                }
            }
            _ => {}
        }
    }
    Ok((
        exact_artifact("vibe", vibe)?,
        exact_artifact("vibe-index", index)?,
    ))
}

fn exact_artifact(name: &str, artifacts: Vec<PathBuf>) -> Result<PathBuf> {
    match artifacts.as_slice() {
        [artifact] => Ok(artifact.clone()),
        [] => bail!("Cargo emitted no executable artifact for `{name}`"),
        _ => bail!(
            "Cargo emitted {} executable artifacts for `{name}`; expected exactly one",
            artifacts.len()
        ),
    }
}

/// Abbreviate a commit hash for a version label (commits are ASCII hex).
pub(crate) fn short_commit(c: &str) -> String {
    c[..c.len().min(10)].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(name: &str, path: &str) -> String {
        serde_json::json!({
            "reason": "compiler-artifact",
            "target": {"name": name, "kind": ["bin"]},
            "executable": path,
        })
        .to_string()
    }

    #[test]
    fn cargo_json_selects_exact_cross_target_artifacts_not_native_guesses() {
        let messages = [
            artifact("unrelated", "target/debug/stale.exe"),
            artifact("vibe", "target/aarch64-unknown-linux-musl/release/vibe"),
            artifact(
                "vibe-index",
                "target/aarch64-unknown-linux-musl/release/vibe-index",
            ),
        ]
        .join("\n");
        let mut diagnostics = Vec::new();
        let (vibe, index) = select_artifacts(messages.as_bytes(), &mut diagnostics).unwrap();
        assert_eq!(
            vibe,
            PathBuf::from("target/aarch64-unknown-linux-musl/release/vibe")
        );
        assert_eq!(
            index,
            PathBuf::from("target/aarch64-unknown-linux-musl/release/vibe-index")
        );
    }

    #[test]
    fn cargo_json_rejects_missing_and_duplicate_essential_artifacts() {
        let only_vibe = artifact("vibe", "cross/vibe");
        assert!(
            select_artifacts(only_vibe.as_bytes(), &mut Vec::new())
                .unwrap_err()
                .to_string()
                .contains("vibe-index")
        );
        let duplicate = [
            artifact("vibe", "cross/vibe-a"),
            artifact("vibe", "cross/vibe-b"),
            artifact("vibe-index", "cross/vibe-index"),
        ]
        .join("\n");
        assert!(
            select_artifacts(duplicate.as_bytes(), &mut Vec::new())
                .unwrap_err()
                .to_string()
                .contains("2 executable artifacts")
        );
    }

    #[test]
    fn cargo_json_preserves_rendered_compiler_diagnostics() {
        let messages = [
            serde_json::json!({
                "reason": "compiler-message",
                "message": {"rendered": "warning: visible diagnostic\n"},
            })
            .to_string(),
            artifact("vibe", "cross/vibe"),
            artifact("vibe-index", "cross/vibe-index"),
        ]
        .join("\n");
        let mut diagnostics = Vec::new();
        select_artifacts(messages.as_bytes(), &mut diagnostics).unwrap();
        assert_eq!(diagnostics, b"warning: visible diagnostic\n");
    }

    #[test]
    fn cargo_build_is_locked_for_every_profile() {
        for profile in [Profile::Debug, Profile::Release] {
            let args = cargo_build_args(profile, false);
            assert!(args.contains(&"--locked"));
            assert!(args.contains(&"--message-format=json-render-diagnostics"));
        }
    }

    /// The offline posture has to reach cargo, or a source rebuild that
    /// issued no request of vibevm's own would still fetch a crate. Online
    /// stays exactly what it was: the flag is added, never assumed.
    #[test]
    fn an_offline_build_passes_the_posture_to_cargo() {
        for profile in [Profile::Debug, Profile::Release] {
            assert!(cargo_build_args(profile, true).contains(&"--offline"));
            assert!(!cargo_build_args(profile, false).contains(&"--offline"));
        }
    }

    #[test]
    fn failure_tail_keeps_cargo_error_but_redacts_external_secrets() {
        let tail = Mutex::new(VecDeque::new());
        let task = Progress::default().task("cargo");
        emit_child_line(
            &task,
            &tail,
            "error[E0425]: fetch https://alice:hunter2@example.test/pkg?token=secret \
             authorization: Bearer also-secret",
        );
        let rendered = diagnostic_tail(&tail);
        assert!(rendered.contains("error[E0425]"), "{rendered}");
        for secret in ["alice", "hunter2", "token=secret", "also-secret"] {
            assert!(!rendered.contains(secret), "leaked {secret}: {rendered}");
        }
        assert!(rendered.contains("[redacted]"), "{rendered}");
    }
}
