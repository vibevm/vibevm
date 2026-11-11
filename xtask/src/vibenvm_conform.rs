//! `cargo xtask vibenvm-conform` — the vibenvm differential conformance
//! suite's entry point (PROP-075 `#VIBENVM-SUITE`, campaign phase P7f).
//!
//! The suite itself is `conformance/vibenvm/run.sh`: a committed, rerunnable
//! script whose **verdict is its exit code** (`0` conformant and completely
//! covered, `1` a differential mismatch, `2` coverage incomplete, `3` a
//! harness error). This shim exists so the gate has the same spelling as
//! every other gate in the repository, and so it keeps that property: it
//! resolves the repo root, optionally re-extracts the inventory from the
//! pinned nvm-sh first, runs the script, and **exits with the script's own
//! status** rather than collapsing every failure into one.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Where the suite lives, relative to the repository root.
const SUITE_DIR: &str = "conformance/vibenvm";

/// Run the suite, then exit this process with its verdict.
///
/// `cases` is passed through as `run.sh --cases <glob>`; `update_inventory`
/// runs `extract-inventory.sh` first, which rewrites the committed
/// `inventory.json` from whatever tag the suite pins — the whole point of
/// re-extraction being that a later nvm release cannot pass unnoticed.
pub(crate) fn run_vibenvm_conform(
    root: &Path,
    cases: Option<&str>,
    update_inventory: bool,
) -> Result<()> {
    let suite = root.join(SUITE_DIR);
    let runner = suite.join("run.sh");
    if !runner.is_file() {
        bail!("no conformance suite at {}", runner.display());
    }
    if !suite.join("mirror").is_dir() {
        bail!(
            "the suite has no local mirror yet: run {}/fetch-mirror.sh once \
             (the only networked step of the suite)",
            SUITE_DIR
        );
    }

    if update_inventory {
        let extract = suite.join("extract-inventory.sh");
        let status = Command::new("bash")
            .arg(&extract)
            .current_dir(&suite)
            .status()
            .with_context(|| format!("cannot run {}", extract.display()))?;
        if !status.success() {
            bail!(
                "extract-inventory.sh failed with {}",
                describe(status.code())
            );
        }
    }

    let mut command = Command::new("bash");
    command.arg(&runner).current_dir(&suite);
    if let Some(glob) = cases {
        command.arg("--cases").arg(glob);
    }
    let status = command
        .status()
        .with_context(|| format!("cannot run {}", runner.display()))?;

    // The verdict is the exit code, so it is propagated verbatim. A signal
    // (no code) is not a verdict: it is reported as the harness error it is.
    match status.code() {
        Some(code) => std::process::exit(code),
        None => bail!("the conformance suite was killed by a signal"),
    }
}

/// A process status for a message: the code, or the fact there wasn't one.
fn describe(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code (killed by a signal)".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shim points at files that exist, with the names the spec and the
    /// campaign plan use. A rename that forgot this module would otherwise
    /// only be found by running the suite.
    #[test]
    fn the_suite_is_where_the_shim_looks_for_it() {
        let root = crate::repo_root().expect("repo root");
        let suite = root.join(SUITE_DIR);
        for name in [
            "run.sh",
            "fetch-mirror.sh",
            "extract-inventory.sh",
            "observe.sh",
            "coverage.py",
            "normalise.rules",
            // `exclusions.toml` is deliberately absent: the accepted-gap
            // set is empty (PROP-075 `#VIBENVM-NO-EXCLUSIONS`), and
            // `coverage.py` treats a missing file as the ordinary state.
            "inventory.json",
            "mirror.lock",
            "nvm-tests.allow",
            "README.md",
        ] {
            assert!(
                suite.join(name).is_file(),
                "the suite is missing {name} at {}",
                suite.display()
            );
        }
        assert!(
            suite.join("cases").is_dir(),
            "the suite has no cases directory"
        );
    }

    /// A missing mirror is reported as something the operator can act on,
    /// and names the one networked step that fixes it, rather than letting
    /// the script fail deep inside its own verification.
    #[test]
    fn a_missing_mirror_is_an_actionable_error() {
        let empty = tempfile::tempdir().expect("tempdir");
        let suite = empty.path().join(SUITE_DIR);
        std::fs::create_dir_all(&suite).expect("create the suite dir");
        std::fs::write(suite.join("run.sh"), "#!/bin/sh\nexit 0\n").expect("write run.sh");
        let error = run_vibenvm_conform(empty.path(), None, false)
            .expect_err("a suite with no mirror cannot run");
        let message = error.to_string();
        assert!(message.contains("fetch-mirror.sh"), "{message}");
    }

    /// Every exclusion row the gate accepts is one of the owner's, and the
    /// committed file is exactly that set — the property `coverage.py`
    /// enforces, asserted here too so `cargo test` says it as well.
    #[test]
    fn the_exclusion_list_is_the_owners_closed_set() {
        let root = crate::repo_root().expect("repo root");
        let text = std::fs::read_to_string(root.join(SUITE_DIR).join("exclusions.toml"))
            .expect("read exclusions.toml");
        // Counted over whole lines, so the file's own prose about the rules
        // is prose and not a row.
        let lines = |wanted: &str| text.lines().filter(|line| *line == wanted).count();
        let rows = lines("[[exclusion]]");
        assert_eq!(
            rows, 14,
            "the owner approved twelve items, which PROP-075 \
             #VIBENVM-EXCLUSIONS-APPROVED's row rule spells as fourteen rows"
        );
        assert_eq!(
            lines("approved_by = \"owner\""),
            rows,
            "every row is approved by the owner"
        );
        assert_eq!(
            lines("approved = \"2026-11-10\""),
            rows,
            "every row carries the owner's decision date"
        );
    }
}
