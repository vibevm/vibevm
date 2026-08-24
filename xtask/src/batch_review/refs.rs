//! C12 — a package reference must name a package that exists.
//!
//! The `git-practices` rename moved four packages to `git-*` names and left
//! every citation behind. B8 marked one of them without noticing, B10 met a
//! second under a different name, and only then did a sweep find the class:
//! **four dead names, 21 canonical files, 33 references — six of them literal
//! `vibe install` command lines in three packages' own READMEs**, each telling
//! a reader to install a name that does not resolve.
//!
//! Nothing checked it, and the check is exact. The denominator is the set of
//! `name = ` values declared in the repository's `vibe.toml` files; the
//! numerator is every `kind:name` reference in the batch's markdown. Both sides
//! are enumerable, so this check has no judgement in it — the same property
//! C11 has and the reason both are worth having.
//!
//! **Scoped to the batch on purpose.** A corpus-wide sweep is a one-off a human
//! runs; this one runs on every batch review and catches the *next* rename
//! while it is still one file.

use std::collections::BTreeSet;
use std::path::Path;

use super::report::Report;

/// The installable kinds a reference may name (PROP-000 §4.1).
const KINDS: &[&str] = &["flow", "feat", "stack", "tool", "mcp", "lang"];

/// Every `name = "…"` declared by a `vibe.toml` under `packages/`.
///
/// Regenerated dependency copies are skipped by path segment, not by
/// substring: this repository's own namespace is `org.vibevm`, so a
/// `contains(".vibe")` filter would delete every package it is meant to find.
/// That mistake was made by hand on 2026-07-27 and is recorded in the WAL.
///
/// The `packages` / `vibedeps` literals are layout-root names kept here
/// because xtask carries no vibe-core edge; the single home of the root
/// names is `crates/vibe-core/src/layout.rs` (PROP-052 L2) — the R4
/// relayout sweep retires this duplication (`vibedeps` is matched by
/// SEGMENT and the move keeps the final segment, so that filter
/// survives the flip unchanged).
pub(super) fn declared_names(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![root.join("packages")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            let seg = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if p.is_dir() {
                if seg == "vibedeps" || seg == ".vibe" || seg == "target" {
                    continue;
                }
                stack.push(p);
            } else if seg == "vibe.toml" {
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue;
                };
                for line in text.lines() {
                    let t = line.trim();
                    if let Some(rest) = t.strip_prefix("name")
                        && let Some(v) = rest.trim_start().strip_prefix('=')
                        && let Some(name) =
                            v.trim().strip_prefix('"').and_then(|s| s.split('"').next())
                    {
                        out.insert(name.to_string());
                        break; // the first `name =` is the package's own
                    }
                }
            }
        }
    }
    out
}

/// Every `kind:name` reference in the text, with its line number.
///
/// A reference may be written qualified (`flow:org.vibevm.world/wal`) or bare
/// (`flow:wal`); the group is stripped so both compare against the same set.
/// Trailing punctuation is trimmed — `flow:decision-records.` at the end of a
/// sentence names the package, not a package with a full stop in it, and a
/// first attempt at this sweep reported exactly that as a dead reference.
///
/// **Nothing is blanked, and that is measured rather than assumed.** Unlike
/// every other check here, this one reads raw text. The corpus writes its
/// citations inside inline code spans — `` `flow:wal` `` — so blanking spans
/// would hide almost every reference; and the six sharpest real defects are
/// `vibe install flow:…` lines inside fenced `bash` blocks, so blanking fences
/// would hide exactly the worst ones. A fenced command a reader is told to run
/// is a reference like any other. The cost is a possible false positive on an
/// illustrative placeholder, which fails loudly with a file and a line — the
/// cheap direction.
pub(super) fn references(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (n, line) in text.split('\n').enumerate() {
        for kind in KINDS {
            let pat = format!("{kind}:");
            let mut from = 0usize;
            while let Some(at) = line[from..].find(&pat) {
                let start = from + at + pat.len();
                let rest = &line[start..];
                let end = rest
                    .find(|c: char| {
                        !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
                    })
                    .unwrap_or(rest.len());
                let raw = &rest[..end];
                from = start + end.max(1);
                let name = raw.rsplit('/').next().unwrap_or(raw).trim_end_matches('.');
                if !name.is_empty() && name.contains(|c: char| c.is_ascii_alphabetic()) {
                    out.push((n + 1, name.to_string()));
                }
            }
        }
    }
    out
}

/// C12 — every package reference in the batch names a declared package.
///
/// `known_dead` is a ratchet, the idiom conform already uses here: names that
/// are dead, **filed**, and deliberately not fixed yet are reported as known
/// and do not fail the check. Without it this would go red on every batch that
/// touches one of F-097's 21 files, and a check that fails on something the
/// reviewer has already decided not to act on is a check the reviewer learns to
/// skip. **New dead names still fail** — which is exactly what the briefs tell
/// each executor: a fifth name would be a new finding.
pub(super) fn c12_package_refs(
    files: &[String],
    root: &Path,
    known_dead: &BTreeSet<String>,
    r: &mut Report,
) {
    let declared = declared_names(root);
    if declared.is_empty() {
        // A zero denominator must never read as clean, for the third time in
        // this tool -- see C1's empty scope and C11's empty tasks directory.
        r.fail(
            "C12 package refs",
            "no package names found under packages/ -- refusing a zero denominator",
        );
        return;
    }
    let mut dead: Vec<String> = Vec::new();
    let mut known = 0usize;
    let mut total = 0usize;
    for f in files {
        let Ok(text) = std::fs::read_to_string(root.join(f)) else {
            continue;
        };
        for (line, name) in references(&text) {
            total += 1;
            if declared.contains(&name) {
                continue;
            }
            if known_dead.contains(&name) {
                known += 1;
                continue;
            }
            dead.push(format!("{f}:{line}  {name}"));
        }
    }
    let tail = if known > 0 {
        format!(" ({known} known-dead, filed)")
    } else {
        String::new()
    };
    if dead.is_empty() {
        r.ok(
            "C12 package refs",
            format!("{total} reference(s), no undeclared name{tail}"),
        );
    } else {
        r.fail(
            "C12 package refs",
            format!(
                "{} of {total} reference(s) name no declared package:\n     {}",
                dead.len(),
                dead.join("\n     ")
            ),
        );
    }
}

// ------------------------------------------------------------- controls
#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(body: &str) -> (tempfile::TempDir, Vec<String>) {
        // The `packages` dir mirrors the walk root of `declared_names`
        // above — a test scaffold, with the layout name's single home in
        // `crates/vibe-core/src/layout.rs` (PROP-052 L2).
        let d = tempfile::tempdir().unwrap();
        let pkg = d
            .path()
            .join("packages")
            .join("org.vibevm.world")
            .join("wal");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("vibe.toml"),
            "[package]\nname = \"wal\"\ngroup = \"x\"\n",
        )
        .unwrap();
        let other = d
            .path()
            .join("packages")
            .join("org.vibevm.world")
            .join("git-atomic-commits");
        // (same `packages` scaffold as above — see the note there)
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(
            other.join("vibe.toml"),
            "[package]\nname = \"git-atomic-commits\"\n",
        )
        .unwrap();
        std::fs::write(d.path().join("probe.md"), body).unwrap();
        (d, vec!["probe.md".to_string()])
    }

    #[test]
    fn a_live_reference_passes() {
        let (d, files) =
            scratch("# T {#root}\n\n##A See `flow:wal` for the checkpoint. @impl/done\n");
        let mut r = Report::default();
        c12_package_refs(&files, d.path(), &BTreeSet::new(), &mut r);
        assert!(!r.failed(), "a live reference went red: {:?}", r.caught());
    }

    /// The failure that happened, four times, and was found two batches late.
    #[test]
    fn a_renamed_package_is_caught() {
        let (d, files) = scratch("# T {#root}\n\n##A Composes `flow:atomic-commits`. @impl/done\n");
        let mut r = Report::default();
        c12_package_refs(&files, d.path(), &BTreeSet::new(), &mut r);
        assert!(r.failed(), "a dead reference must fail");
    }

    #[test]
    fn a_qualified_reference_resolves_by_its_last_segment() {
        let (d, files) = scratch("##A `flow:org.vibevm.world/wal` is installed. @impl/done\n");
        let mut r = Report::default();
        c12_package_refs(&files, d.path(), &BTreeSet::new(), &mut r);
        assert!(!r.failed(), "qualified form went red: {:?}", r.caught());
    }

    /// A first attempt reported `decision-records.` as dead because the
    /// sentence ended there. Trailing punctuation is not part of a name.
    #[test]
    fn a_trailing_full_stop_is_not_part_of_the_name() {
        let refs = references("see flow:wal.\n");
        assert_eq!(refs, vec![(1, "wal".to_string())], "{refs:?}");
    }

    /// The six sharpest real instances of this defect are `vibe install` lines
    /// inside fenced `bash` blocks, in three packages' own READMEs. A fenced
    /// command a reader is told to run is a reference like any other — which is
    /// why this is the only check here that blanks nothing.
    ///
    /// *This control originally asserted the opposite, on the assumption that a
    /// fenced reference is an illustration. Measuring the corpus refuted it.*
    #[test]
    fn a_dead_reference_inside_a_fenced_command_is_caught() {
        let (d, files) = scratch(
            "# T {#root}\n\n## Install\n\n```bash\nvibe install flow:atomic-commits\n```\n",
        );
        let mut r = Report::default();
        c12_package_refs(&files, d.path(), &BTreeSet::new(), &mut r);
        assert!(
            r.failed(),
            "a fenced install line naming a dead package must fail"
        );
    }

    #[test]
    fn a_live_reference_inside_a_fenced_command_passes() {
        let (d, files) = scratch("# T {#root}\n\n```bash\nvibe install flow:wal\n```\n");
        let mut r = Report::default();
        c12_package_refs(&files, d.path(), &BTreeSet::new(), &mut r);
        assert!(
            !r.failed(),
            "a live fenced install line went red: {:?}",
            r.caught()
        );
    }

    /// A filed name is reported and does not fail; an unfiled one still does.
    #[test]
    fn a_filed_dead_name_is_known_and_a_new_one_is_not() {
        let (d, files) = scratch(
            "##A Composes `flow:atomic-commits`. @impl/done
",
        );
        let filed: BTreeSet<String> = ["atomic-commits".to_string()].into_iter().collect();
        let mut r = Report::default();
        c12_package_refs(&files, d.path(), &filed, &mut r);
        assert!(!r.failed(), "a filed name must not fail: {:?}", r.caught());

        let (d2, files2) = scratch(
            "##A Composes `flow:some-new-rename`. @impl/done
",
        );
        let mut r2 = Report::default();
        c12_package_refs(&files2, d2.path(), &filed, &mut r2);
        assert!(r2.failed(), "an unfiled name must still fail");
    }

    #[test]
    fn an_empty_denominator_refuses_rather_than_passing() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("probe.md"), "##A `flow:wal` @impl/done\n").unwrap();
        let mut r = Report::default();
        c12_package_refs(
            &["probe.md".to_string()],
            d.path(),
            &BTreeSet::new(),
            &mut r,
        );
        assert!(r.failed(), "a zero denominator must never read as clean");
    }
}
