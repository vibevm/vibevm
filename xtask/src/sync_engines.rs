//! `cargo xtask sync-engines` — the vendor-sync gate for the neutral
//! discipline engines (DEFERRALS-CLOSEOUT-PLAN v0.1, D1).
//!
//! The language-neutral engine crates are AUTHORED in
//! `flow:org.vibevm.ai-native/core-ai-native` — the flow package owns the engine —
//! and each language stack ships a byte-identical VENDORED copy under its
//! own `crates/vendor/`. A Cargo path-dep cannot cross package slots: the
//! authored layout (`packages/org.vibevm/<name>/v<ver>/`) and the
//! materialised layout (`vibedeps/<group>.<name>/<ver>/`) disagree on both
//! directory naming and version prefix, and every slot must stay a
//! self-buildable workspace (PROP-024 §2.4). Vendoring keeps that
//! property; this gate makes divergence mechanically impossible.
//!
//! `sync-engines` mirrors the authored crates into every target
//! (incrementally — only differing files are rewritten, extras are
//! removed, so cargo's mtime-based rebuilds stay quiet); `--check`
//! byte-compares and exits non-zero on drift. `tools/self-check.sh` runs
//! the check, so an edit to a vendored copy cannot land silently — the
//! fix surface is always the authored crate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use walkdir::WalkDir;

use crate::repo_root;

/// `sync-engines.toml` at the repo root — which crates flow from where
/// to where, as `[[sync]]` sets (MCP-SOVEREIGNTY D3a: the mcp packages
/// vendor stack-authored crates too, so one source_root stopped being
/// enough). Version-bearing paths live here (not in code) so a package
/// version bump edits data, not the tool.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SyncManifest {
    /// The sync sets, each mirrored independently.
    pub sync: Vec<SyncSet>,
    /// Vendored engine dirs that are deliberately the target of NO sync
    /// set: superseded version slots. A frozen slot ships the engine of
    /// its own era, and syncing today's sources into it would rewrite
    /// history — the exact opposite of what a version slot is. Every
    /// entry is a recorded decision the coverage check accepts; a
    /// pattern that stops matching a real dir is reported so the list
    /// cannot rot (same law as conform.toml's dead-exclusion warning).
    #[serde(default)]
    pub frozen_targets: Vec<String>,
}

/// One authored-source → vendored-targets set.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SyncSet {
    /// Directory holding the AUTHORED crates, repo-relative.
    pub source_root: String,
    /// Crate directory names under `source_root` to mirror.
    pub crates: Vec<String>,
    /// Vendor directories (repo-relative) that receive the copies.
    pub targets: Vec<String>,
}

const MANIFEST: &str = "sync-engines.toml";

/// The AI-Native package family this gate is the denominator for.
///
/// A vendored engine copy lives at `<slot>/crates/vendor/<crate>` — the
/// layout law this file's header states — so every such directory in
/// the family must be the target of some `[[sync]]` set. Until F-086
/// this file only ever counted the pairs it was *told* about, and a
/// count with no denominator always agrees with itself: two published
/// go packages shipped an engine older than their own manifest declared
/// and the gate stayed green through every commit that caused it.
///
/// A family name, deliberately not a version: a version literal here is
/// exactly the rot this denominator exists to catch.
///
/// The `packages/` prefix is a layout-root literal kept here because
/// xtask carries no vibe-core edge; the single home of the root names
/// is `crates/vibe-core/src/layout.rs` (PROP-052 L2) — the R4 relayout
/// sweep retires this duplication.
const FAMILY_ROOT: &str = "vibevm/vibepacks/org.vibevm.ai-native";

/// Directories the enumeration below never descends into. Build output
/// and VCS state are not content (same reasoning as `file_set`);
/// `vibedeps/` and `.vibe/cache/` hold RESOLVED dependency copies that a
/// resolver writes and rewrites, so they are never an authored sync
/// target and must not count against the denominator.
///
/// `vibedeps` is the deps-slot dir name (a layout name; single home
/// `crates/vibe-core/src/layout.rs`, PROP-052 L2 — matched by SEGMENT
/// here, and the R4 move keeps the final segment, so the filter
/// survives the flip unchanged).
const SCAN_DENY: [&str; 5] = ["target", ".git", "node_modules", ".vibe", "vibedeps"];

/// Every directory under [`FAMILY_ROOT`] that holds vendored engine
/// copies, repo-relative and `/`-separated so it compares against a
/// manifest `target` verbatim on any platform.
fn vendored_dirs(root: &Path) -> Result<BTreeSet<String>> {
    let family = root.join(FAMILY_ROOT);
    let mut dirs = BTreeSet::new();
    if !family.is_dir() {
        return Ok(dirs);
    }
    for entry in WalkDir::new(&family)
        .into_iter()
        .filter_entry(|e| !SCAN_DENY.iter().any(|d| e.file_name() == *d))
    {
        let entry =
            entry.with_context(|| format!("sync-engines: walking `{}`", family.display()))?;
        if !entry.file_type().is_dir() || entry.file_name() != "vendor" {
            continue;
        }
        // `crates/vendor` is the layout law; a `vendor` dir anywhere
        // else is somebody's unrelated directory, not an engine home.
        let under_crates = entry
            .path()
            .parent()
            .and_then(|p| p.file_name())
            .is_some_and(|n| n == "crates");
        if !under_crates {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .with_context(|| "sync-engines: path outside the repo root")?;
        dirs.insert(rel.to_string_lossy().replace('\\', "/"));
    }
    Ok(dirs)
}

/// The vendored-engine directories no `[[sync]]` set targets — the gap
/// a bare pair count can never show. Returns `(uncovered, total)`.
fn uncovered_vendor_dirs(root: &Path, manifest: &SyncManifest) -> Result<(Vec<String>, usize)> {
    let declared: BTreeSet<String> = manifest
        .sync
        .iter()
        .flat_map(|set| set.targets.iter())
        .map(|target| target.replace('\\', "/"))
        .collect();
    let frozen: BTreeSet<String> = manifest
        .frozen_targets
        .iter()
        .map(|target| target.replace('\\', "/"))
        .collect();
    let found = vendored_dirs(root)?;
    let total = found.len();
    // A frozen entry that names no real dir is a rotten record — say so
    // (the dead-exclusion law), but do not fail the run over it.
    for dead in frozen.iter().filter(|f| !found.contains(*f)) {
        eprintln!(
            "sync-engines: frozen_targets names `{dead}`, which holds no vendored \
             engine copies — delete the stale entry."
        );
    }
    let uncovered = found
        .into_iter()
        .filter(|dir| !declared.contains(dir) && !frozen.contains(dir))
        .collect();
    Ok((uncovered, total))
}

fn load_manifest(root: &Path) -> Result<SyncManifest> {
    let path = root.join(MANIFEST);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("sync-engines: reading `{}`", path.display()))?;
    let manifest: SyncManifest =
        toml::from_str(&text).with_context(|| format!("sync-engines: parsing `{MANIFEST}`"))?;
    if manifest.sync.is_empty() {
        bail!("sync-engines: `{MANIFEST}` declares no [[sync]] sets");
    }
    for set in &manifest.sync {
        if set.crates.is_empty() || set.targets.is_empty() {
            bail!(
                "sync-engines: the `{}` set names no crates or no targets",
                set.source_root
            );
        }
    }
    Ok(manifest)
}

/// Relative paths of every file under `dir`, sorted. Build output,
/// VCS state, and resolved installs never count as content — the same
/// denylist as PROP-024 §2.2's shippable tree, so a mirrored dir can
/// never smuggle what an install would exclude (node_modules bit once:
/// the TS stack's embedded-source tools/ carries a local install for
/// its own tests, and the first mirror copied it wholesale).
fn file_set(dir: &Path) -> Result<BTreeSet<PathBuf>> {
    const DENY: [&str; 4] = ["target", ".git", "node_modules", ".vibe"];
    let mut files = BTreeSet::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !DENY.iter().any(|d| e.file_name() == *d))
    {
        let entry = entry.with_context(|| format!("sync-engines: walking `{}`", dir.display()))?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)
                .with_context(|| "sync-engines: path outside its walk root")?;
            files.insert(rel.to_path_buf());
        }
    }
    Ok(files)
}

/// One crate's drift between its authored source and one vendored copy.
#[derive(Debug, Default)]
struct CrateDrift {
    missing: Vec<PathBuf>,
    extra: Vec<PathBuf>,
    changed: Vec<PathBuf>,
}

impl CrateDrift {
    fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty() && self.changed.is_empty()
    }
}

fn diff_crate(src: &Path, dst: &Path) -> Result<CrateDrift> {
    let src_files = file_set(src)?;
    let dst_files = file_set(dst)?;
    let mut drift = CrateDrift::default();
    for rel in src_files.union(&dst_files) {
        match (src_files.contains(rel), dst_files.contains(rel)) {
            (true, false) => drift.missing.push(rel.clone()),
            (false, true) => drift.extra.push(rel.clone()),
            (true, true) => {
                let a = fs::read(src.join(rel))
                    .with_context(|| format!("sync-engines: reading `{}`", rel.display()))?;
                let b = fs::read(dst.join(rel)).with_context(|| {
                    format!("sync-engines: reading vendored `{}`", rel.display())
                })?;
                if a != b {
                    drift.changed.push(rel.clone());
                }
            }
            (false, false) => unreachable!("union yields members of at least one set"),
        }
    }
    Ok(drift)
}

/// Mirror `src` into `dst` incrementally: write only differing files,
/// remove extras. Returns the number of filesystem writes/removals.
fn mirror_crate(src: &Path, dst: &Path) -> Result<usize> {
    let drift = diff_crate(src, dst)?;
    let mut ops = 0usize;
    for rel in drift.missing.iter().chain(drift.changed.iter()) {
        let to = dst.join(rel);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("sync-engines: mkdir `{}`", parent.display()))?;
        }
        fs::copy(src.join(rel), &to)
            .with_context(|| format!("sync-engines: copying `{}`", rel.display()))?;
        ops += 1;
    }
    for rel in &drift.extra {
        fs::remove_file(dst.join(rel))
            .with_context(|| format!("sync-engines: removing stale `{}`", rel.display()))?;
        ops += 1;
    }
    Ok(ops)
}

/// The engine shared by the CLI entry and the tests: check or mirror
/// every (target × crate) pair of every set of `manifest` under `root`.
fn sync_all(root: &Path, manifest: &SyncManifest, check: bool) -> Result<(usize, Vec<String>)> {
    let mut ops = 0usize;
    let mut drift_lines = Vec::new();
    let sets_total = manifest.sync.len();
    for (set_no, set) in manifest.sync.iter().enumerate() {
        // Per-set progress (owner request 2026-08-24): a full mirror walks
        // thousands of files and used to look hung; the narration goes to
        // stderr so the summary line on stdout stays the machine surface.
        let started = std::time::Instant::now();
        let ops_before = ops;
        let drift_before = drift_lines.len();
        eprintln!(
            "sync-engines: [{}/{sets_total}] {} → {} target(s) × {} crate(s)…",
            set_no + 1,
            set.source_root,
            set.targets.len(),
            set.crates.len(),
        );
        for target in &set.targets {
            for krate in &set.crates {
                let src = root.join(&set.source_root).join(krate);
                if !src.is_dir() {
                    bail!(
                        "sync-engines: authored crate `{krate}` missing under `{}`",
                        set.source_root
                    );
                }
                let dst = root.join(target).join(krate);
                if check {
                    let drift = diff_crate(&src, &dst)?;
                    if !drift.is_clean() {
                        for rel in &drift.missing {
                            drift_lines
                                .push(format!("{target}/{krate}: missing `{}`", rel.display()));
                        }
                        for rel in &drift.extra {
                            drift_lines
                                .push(format!("{target}/{krate}: extra `{}`", rel.display()));
                        }
                        for rel in &drift.changed {
                            drift_lines
                                .push(format!("{target}/{krate}: differs `{}`", rel.display()));
                        }
                    }
                } else {
                    ops += mirror_crate(&src, &dst)?;
                }
            }
        }
        let secs = started.elapsed().as_secs();
        if check {
            let drift_here = drift_lines.len() - drift_before;
            eprintln!(
                "sync-engines: [{}/{sets_total}] {} — {} ({secs}s)",
                set_no + 1,
                set.source_root,
                if drift_here == 0 {
                    "clean".to_string()
                } else {
                    format!("{drift_here} drift item(s)")
                },
            );
        } else {
            eprintln!(
                "sync-engines: [{}/{sets_total}] {} — {} file op(s) ({secs}s)",
                set_no + 1,
                set.source_root,
                ops - ops_before,
            );
        }
    }
    Ok((ops, drift_lines))
}

/// Total (target × crate) pairs across all sets — the count the CLI
/// reports.
fn pair_count(manifest: &SyncManifest) -> usize {
    manifest
        .sync
        .iter()
        .map(|s| s.crates.len() * s.targets.len())
        .sum()
}

pub(crate) fn run_sync_engines(check: bool) -> Result<()> {
    let root = repo_root()?;
    let manifest = load_manifest(&root)?;
    let (uncovered, vendor_dirs) = uncovered_vendor_dirs(&root, &manifest)?;
    for dir in &uncovered {
        eprintln!(
            "sync-engines: `{dir}` holds vendored engine copies but is the target of no \
             [[sync]] set — its copies drift with nothing watching."
        );
    }
    let (ops, drift) = sync_all(&root, &manifest, check)?;
    let pairs = pair_count(&manifest);
    let sets = manifest.sync.len();
    if check {
        // The denominator first: a package outside the manifest cannot
        // show up as drift, so a clean pair count over the wrong set of
        // packages is the failure this reports (F-086).
        if !uncovered.is_empty() {
            bail!(
                "sync-engines --check: {} of {vendor_dirs} vendored engine dir(s) under \
                 {FAMILY_ROOT}/ are the target of no [[sync]] set (named above). Add each \
                 to `{MANIFEST}` — a package whose engines nothing syncs ships whatever \
                 it was copied with.",
                uncovered.len(),
            );
        }
        if drift.is_empty() {
            let frozen = manifest.frozen_targets.len();
            println!(
                "sync-engines --check: every vendored crate matches its authored source \
                 ({pairs} pair(s) across {sets} sync set(s)); {vendor_dirs} vendored \
                 engine dir(s) under {FAMILY_ROOT}/ accounted for ({} sync targets, \
                 {frozen} recorded frozen slots).",
                vendor_dirs - frozen,
            );
            return Ok(());
        }
        for line in &drift {
            eprintln!("sync-engines: {line}");
        }
        bail!(
            "sync-engines --check: {} drift item(s) across {pairs} vendored pair(s). \
             Edit the AUTHORED copy under the set's source_root, then run \
             `cargo xtask sync-engines`; vendored copies are write-throughs, \
             never a fix surface.",
            drift.len(),
        );
    }
    println!(
        "sync-engines: {ops} file operation(s); {pairs} vendored pair(s) across {sets} \
         sync set(s) mirror their authored sources; {} of {vendor_dirs} vendored engine \
         dir(s) under {FAMILY_ROOT}/ are covered.",
        vendor_dirs - uncovered.len(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, body).expect("write");
    }

    fn manifest() -> SyncManifest {
        SyncManifest {
            frozen_targets: Vec::new(),
            sync: vec![SyncSet {
                source_root: "core/crates".into(),
                crates: vec!["engine".into()],
                targets: vec![
                    "stack-a/crates/vendor".into(),
                    "stack-b/crates/vendor".into(),
                ],
            }],
        }
    }

    #[test]
    fn mirror_writes_updates_and_removes_extras_then_check_is_clean() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            root,
            "core/crates/engine/Cargo.toml",
            "[package]\nname = \"engine\"\n",
        );
        write(root, "core/crates/engine/src/lib.rs", "pub fn one() {}\n");
        write(root, "stack-a/crates/vendor/engine/src/stale.rs", "gone\n");

        let m = manifest();
        let (ops, drift) = sync_all(root, &m, false).expect("sync");
        assert!(drift.is_empty());
        // 2 files x 2 targets written + 1 stale removal.
        assert_eq!(ops, 5);
        let vendored = root.join("stack-b/crates/vendor/engine/src/lib.rs");
        assert_eq!(
            fs::read_to_string(vendored).expect("read"),
            "pub fn one() {}\n"
        );
        assert!(
            !root
                .join("stack-a/crates/vendor/engine/src/stale.rs")
                .exists()
        );

        let (_, drift) = sync_all(root, &m, true).expect("check");
        assert!(drift.is_empty(), "{drift:?}");

        // A second mirror run is a no-op: nothing rewritten, mtimes quiet.
        let (ops, _) = sync_all(root, &m, false).expect("re-sync");
        assert_eq!(ops, 0);
    }

    #[test]
    fn check_reports_changed_missing_and_extra_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "core/crates/engine/src/lib.rs", "pub fn one() {}\n");
        let m = manifest();
        sync_all(root, &m, false).expect("sync");

        write(
            root,
            "stack-a/crates/vendor/engine/src/lib.rs",
            "tampered\n",
        );
        write(root, "stack-a/crates/vendor/engine/src/extra.rs", "extra\n");
        write(root, "core/crates/engine/src/new.rs", "pub fn two() {}\n");

        let (_, drift) = sync_all(root, &m, true).expect("check");
        let joined = drift.join("\n");
        assert!(
            joined.contains("differs `src\\lib.rs`") || joined.contains("differs `src/lib.rs`")
        );
        assert!(
            joined.contains("extra `src\\extra.rs`") || joined.contains("extra `src/extra.rs`")
        );
        // `new.rs` is missing from BOTH targets — one drift line each.
        assert_eq!(
            drift.iter().filter(|l| l.contains("missing")).count(),
            2,
            "{drift:?}"
        );
    }

    /// The denominator: a slot whose engines nothing syncs is named,
    /// and naming it is the whole point — the pair count over the sets
    /// that ARE declared stays clean either way.
    #[test]
    fn an_untargeted_vendor_dir_is_named_and_counted() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let covered = format!("{FAMILY_ROOT}/covered-lang/v0.1.0/crates/vendor");
        let orphan = format!("{FAMILY_ROOT}/orphan-lang/v0.1.0/crates/vendor");
        write(
            root,
            &format!("{covered}/engine/src/lib.rs"),
            "pub fn a() {}\n",
        );
        write(
            root,
            &format!("{orphan}/engine/src/lib.rs"),
            "pub fn a() {}\n",
        );

        let m = SyncManifest {
            frozen_targets: Vec::new(),
            sync: vec![SyncSet {
                source_root: "core/crates".into(),
                crates: vec!["engine".into()],
                targets: vec![covered.clone()],
            }],
        };
        let (uncovered, total) = uncovered_vendor_dirs(root, &m).expect("scan");
        assert_eq!(total, 2, "both vendor dirs are in the denominator");
        assert_eq!(uncovered, vec![orphan.clone()]);

        // Declaring it closes the gap — nothing else about the manifest
        // changed, which is exactly why a pair count could never see it.
        let m = SyncManifest {
            frozen_targets: Vec::new(),
            sync: vec![SyncSet {
                source_root: "core/crates".into(),
                crates: vec!["engine".into()],
                targets: vec![covered, orphan],
            }],
        };
        let (uncovered, total) = uncovered_vendor_dirs(root, &m).expect("rescan");
        assert!(uncovered.is_empty(), "{uncovered:?}");
        assert_eq!(total, 2);
    }

    /// Resolved dependency copies are not authored slots: `vibedeps/`
    /// and `.vibe/cache/` carry whole vendored trees a resolver wrote,
    /// and counting them would make the gate demand sync sets for
    /// generated directories. A `vendor` dir that is not a child of
    /// `crates/` is somebody else's directory, likewise.
    #[test]
    fn resolved_copies_and_stray_vendor_dirs_stay_out_of_the_denominator() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        for rel in [
            "real-lang/v0.1.0/vibedeps/org.x.x/0.1.0/crates/vendor/engine/src/lib.rs",
            "real-lang/v0.1.0/.vibe/cache/org.x/y/v0.1.0/crates/vendor/engine/src/lib.rs",
            "real-lang/v0.1.0/target/debug/crates/vendor/engine/src/lib.rs",
            "real-lang/v0.1.0/tools/vendor/thing.js",
        ] {
            write(root, &format!("{FAMILY_ROOT}/{rel}"), "x\n");
        }
        let (uncovered, total) = uncovered_vendor_dirs(root, &manifest()).expect("scan");
        assert_eq!(total, 0, "nothing above is an authored engine home");
        assert!(uncovered.is_empty(), "{uncovered:?}");
    }

    #[test]
    fn build_output_never_counts_as_crate_content() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "core/crates/engine/src/lib.rs", "pub fn one() {}\n");
        write(root, "core/crates/engine/target/debug/junk", "junk\n");
        let m = manifest();
        sync_all(root, &m, false).expect("sync");
        assert!(!root.join("stack-a/crates/vendor/engine/target").exists());
        let (_, drift) = sync_all(root, &m, true).expect("check");
        assert!(drift.is_empty(), "{drift:?}");
    }
}
