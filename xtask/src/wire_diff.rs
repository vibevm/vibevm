//! `cargo xtask wire-diff` — the epoch verdict over the golden corpora
//! (PROP-044 §4.7 `##M-BREAK-WINDOW`): the gate does not forbid breaks —
//! it makes an *unannounced* break impossible. Breaking in this project
//! is lawful and cheap; breaking unnoticed is not. The corpus supplies
//! the etalon, `formats/EPOCHS.toml` supplies the regime, and this verb
//! is the step that joins them into a verdict.
//!
//! Three questions, each with exactly one instrument:
//!
//! 1. *Is each corpus the projection of its journal?* Reused, not
//!    reimplemented: [`crate::rebuild::run_rebuild`] under `--check` is
//!    the one projector. A second projector could drift from the first,
//!    and the divergence would surface as a gate forbidding what a
//!    report had just allowed. Drift here is red under every regime —
//!    a derived artifact holding truth (`##FORBID-SECRET-TRUTH`) is not
//!    an epoch matter.
//! 2. *Did the watched wire surface shift relative to the COMMIT?* `git
//!    diff --exit-code --name-only HEAD -- schemas/ formats/` — the
//!    `check-codegen` form, widened to the whole surface the break-window
//!    promise names. A separate question from (1): (1) asks whether the
//!    catalog is a projection of its journal, (2) whether schemas,
//!    corpora, or other format declarations moved since the last commit.
//!    The corpus's authored half counts too — the `state/journal/` shards
//!    ARE the journal format's golden bytes, and they are compared against
//!    the commit, never reprojected.
//! 3. *What does the regime say?* [`crate::epochs::Epochs::load`] — the
//!    one loader; no reading of the flags goes around it.
//!
//! The verdict table (§ the law, PROP-044 §4.7):
//!
//! | `public` | window | watched-surface diff | verdict |
//! |----------|--------|-------------|---------|
//! | `false` | any | empty | green, one line |
//! | `false` | any | non-empty | green, REPORTING — names what shifted |
//! | `true` | closed | non-empty | red: the window rejects it |
//! | `true` | open | non-empty, no fresh note | red: break not declared |
//! | `true` | open | non-empty + fresh note | green |
//!
//! A **fresh note** is a file under `formats/breaks/` that git sees as
//! added in the current change — untracked (`??`) or staged-add (`A…`)
//! in `git status --porcelain`. A note from an earlier change is
//! tracked and clean, invisible to that probe, and can therefore never
//! be credited as the announcement of a new break: the probe answers
//! «was a note added NOW», not «do notes exist».
//!
//! The reporting row is a position, not leniency: until the owner
//! declares the first public presentation, breaking is free and
//! unmigrated, so the gate SPEAKS — names the shifted bytes, says the
//! note is optional (D13) — and states in plain text that green does
//! not mean «nothing changed», so nobody reads the exit code as an
//! all-clear it never was.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::codegen::format_id::load_format_registry;
use crate::epochs::Epochs;
use crate::rebuild;

/// The five-row verdict table, as a pure function of the regime and the
/// two facts git established — so the tests walk exactly the rows the
/// operator meets, and the git probes stay thin.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Nothing shifted — one green line, at any flag state.
    Quiet,
    /// Pre-publication shift — green, but it SPEAKS.
    Reporting,
    /// `public = true`, window closed — red.
    ClosedWindow,
    /// `public = true`, window open, shift undeclared — red.
    Undeclared,
    /// `public = true`, window open, shift declared by a fresh note.
    Declared,
}

/// The operator-facing classes inside the watched wire surface. Keeping
/// schema shifts distinct from corpus shifts matters: a changed schema with
/// no regenerated corpus needs a different repair from bytes that moved with
/// their declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShiftClass {
    Schema,
    Corpus,
    OtherFormat,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ShiftCounts {
    schemas: usize,
    corpora: usize,
    other_formats: usize,
}

fn verdict(epochs: &Epochs, shifted: bool, declared: bool) -> Verdict {
    if !shifted {
        Verdict::Quiet
    } else if !epochs.public {
        Verdict::Reporting
    } else if !epochs.break_window_open {
        Verdict::ClosedWindow
    } else if declared {
        Verdict::Declared
    } else {
        Verdict::Undeclared
    }
}

/// Classify a repository-relative path against the exact perimeter promised
/// by the break-window gate: everything below `schemas/` and `formats/`.
fn shift_class(path: &str) -> Option<ShiftClass> {
    let path = Path::new(path);
    if path.starts_with("schemas") {
        Some(ShiftClass::Schema)
    } else if path.starts_with("formats/corpora") {
        Some(ShiftClass::Corpus)
    } else if path.starts_with("formats") {
        Some(ShiftClass::OtherFormat)
    } else {
        None
    }
}

fn shift_counts(shifted: &[String]) -> ShiftCounts {
    let mut counts = ShiftCounts::default();
    for file in shifted {
        match shift_class(file) {
            Some(ShiftClass::Schema) => counts.schemas += 1,
            Some(ShiftClass::Corpus) => counts.corpora += 1,
            Some(ShiftClass::OtherFormat) => counts.other_formats += 1,
            None => {}
        }
    }
    counts
}

/// Lines shared by every speaking verdict. The existing per-path `shifted`
/// class remains stable; the summary makes a schema-only shift unmistakable
/// and keeps its totals mechanically consistent with those path lines.
fn shift_detail_lines(shifted: &[String]) -> Vec<String> {
    let counts = shift_counts(shifted);
    let mut lines = vec![format!(
        "wire-diff: shift classes — schema: {}, corpus: {}, other formats: {}",
        counts.schemas, counts.corpora, counts.other_formats
    )];
    if counts.schemas > 0 && counts.corpora == 0 {
        lines.push(format!(
            "wire-diff: schema changed without a corpus move — {} schema path(s), 0 corpus path(s)",
            counts.schemas
        ));
    }
    lines.extend(
        shifted
            .iter()
            .map(|file| format!("  wire-diff: shifted `{file}`")),
    );
    lines
}

/// The distinct corpus homes the registry names — every `[format.*]`
/// record whose `corpus` is not `"none"`, de-duplicated (many formats
/// share one home) and sorted (deterministic across platforms). The
/// registry, not a directory walk, is the denominator: the registry's
/// own comment says a record whose bytes the corpus carries must name
/// it, or the wire-diff has nowhere to look; a walk cannot tell a
/// corpus from a stray directory and would silently widen the gate.
fn corpus_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in load_format_registry(root)? {
        if entry.corpus == "none" {
            continue;
        }
        let dir = PathBuf::from(&entry.corpus);
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// The schema and format files git sees as shifted vs the commit — `None`
/// when the committed wire surface and the working tree agree. `HEAD` makes
/// both staged and unstaged tracked changes count; a wholly untracked new file
/// does not. `--name-only` lets every speaking verdict name what moved.
fn wire_shift(root: &Path) -> Result<Option<Vec<String>>> {
    let out = Command::new("git")
        .args([
            "diff",
            "--exit-code",
            "--name-only",
            "HEAD",
            "--",
            "schemas/",
            "formats/",
        ])
        .current_dir(root)
        .output()
        .context("wire-diff: spawning `git diff` over schemas/ and formats/")?;
    match out.status.code() {
        Some(0) => Ok(None),
        Some(1) => Ok(Some(
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_string)
                .collect(),
        )),
        code => bail!(
            "wire-diff: `git diff --name-only HEAD -- schemas/ formats/` failed \
             (exit {code:?}): {}. The shift probe needs a working git over \
             this tree; without it the verdict has no second question to \
             answer.",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
    }
}

/// The break notes git sees as added in the current change — untracked
/// or staged-add under `formats/breaks/`, per `git status --porcelain`.
/// This is the «new note» test the verdict table demands: it cannot
/// credit an OLD note, because an old note is tracked and clean, hence
/// invisible to `git status`. A modified old note (` M`) does not count
/// either — editing yesterday's announcement is not announcing today's
/// break.
fn fresh_break_notes(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .args(["status", "--porcelain", "--", "formats/breaks/"])
        .current_dir(root)
        .output()
        .context("wire-diff: spawning `git status` over formats/breaks/")?;
    if !out.status.success() {
        bail!(
            "wire-diff: `git status --porcelain -- formats/breaks/` failed \
             ({}): {}. The note probe needs a working git over this tree.",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let mut notes = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some((status, path)) = porcelain_line(line) else {
            continue;
        };
        if status.contains('?') || status.starts_with('A') {
            notes.push(path.to_string());
        }
    }
    Ok(notes)
}

/// Split one `git status --porcelain` line into its two-letter status
/// and the path. A rename line (`R  old -> new`) credits the NEW path —
/// the side being added now. Lines without the `XY ` prefix (none are;
/// defensively) yield `None`.
fn porcelain_line(line: &str) -> Option<(&str, &str)> {
    let status = line.get(0..2)?;
    let path = line.get(3..)?;
    let path = match path.rsplit_once(" -> ") {
        Some((_old, new)) => new,
        None => path,
    };
    Some((status, path))
}

pub(crate) fn run_wire_diff() -> Result<()> {
    let root = crate::repo_root()?;

    // (1) Every PROJECTED corpus is its journal's projection — the
    // rebuild projector, reused. It refuses loudly on drift, under
    // every regime: that question has no epoch defence.
    //
    // Two corpus genres live under `formats/corpora/` since the B-079
    // envelope mints, told apart by what they carry. A corpus with a
    // `state/journal/` subtree is a PROJECTION (the catalog): its
    // integrity IS "reproject and byte-compare", which is this step.
    // A corpus without one is AUTHORED GOLDEN DOCUMENTS (the CLI/HTTP
    // envelopes): there is no journal to fold and nothing to rebuild —
    // its integrity lives in its own wire-parity oracle (writer emits
    // the corpus bytes, generated reader round-trips them) and in the
    // shift probe below, which watches every corpus alike.
    let corpora = corpus_dirs(&root)?;
    if corpora.is_empty() {
        bail!(
            "wire-diff: `formats/REGISTRY.toml` names no corpus (`corpus = \
             \"none\"` in every record) — the verdict has nothing to stand \
             on. A wire-diff over no corpora is a gate that cannot fail, \
             which is the disease a denominator exists to catch.\n\
             Fix: point the changed format's `corpus` at its golden-bytes \
             home, then re-run `cargo xtask wire-diff`."
        );
    }
    let mut projected = 0usize;
    for dir in &corpora {
        let corpus_root = root.join(dir);
        if corpus_root.join("state").join("journal").is_dir() {
            rebuild::run_rebuild(true, &corpus_root)?;
            projected += 1;
        }
    }

    // (2) The watched wire surface vs the commit, (3) the regime and the note.
    let shifted: Vec<String> = wire_shift(&root)?.unwrap_or_default();
    let epochs = Epochs::load(&root)?;
    let notes = fresh_break_notes(&root)?;

    match verdict(&epochs, !shifted.is_empty(), !notes.is_empty()) {
        Verdict::Quiet => {
            println!(
                "wire-diff: {projected} projected corpus home(s) proven against \
                 their journals, {} authored golden home(s) watched by the shift \
                 probe; no tracked path under `schemas/` or `formats/` shifted vs \
                 the commit — nothing to declare.",
                corpora.len() - projected
            );
            Ok(())
        }
        Verdict::Reporting => {
            println!(
                "wire-diff: REPORTING (green, exit 0) — the watched wire surface \
                 shifted vs the commit while `public = false`:"
            );
            for line in shift_detail_lines(&shifted) {
                println!("{line}");
            }
            println!(
                "wire-diff: the pre-publication regime — breaking is free and \
                 unmigrated; a break note under `formats/breaks/` is OPTIONAL \
                 until the owner declares the first public presentation (D13, \
                 PROP-044 §7)."
            );
            println!(
                "wire-diff: green here does NOT mean «nothing changed» — it \
                 means the change is allowed unannounced. The paths above ARE \
                 different from the commit."
            );
            Ok(())
        }
        Verdict::ClosedWindow => {
            for line in shift_detail_lines(&shifted) {
                eprintln!("{line}");
            }
            bail!(
                "wire-diff: RED — the break window is CLOSED.\n\
                 rule: `public = true` with `break_window_open = false` \
                 rejects changes under `schemas/**` and `formats/**` outright \
                 (PROP-044 §4.7 ##M-BREAK-WINDOW).\n\
                 why: a closed window is an owner decision — a period of \
                 stability is the state of this flag, and this change rides \
                 past it.\n\
                 fix: restore the named schema/format paths to drop the change, \
                 or have the owner reopen the window in `formats/EPOCHS.toml` \
                 — a wire change never lands through a closed window."
            );
        }
        Verdict::Undeclared => {
            for line in shift_detail_lines(&shifted) {
                eprintln!("{line}");
            }
            bail!(
                "wire-diff: RED — the break is not declared.\n\
                 rule: a change under `schemas/**` or `formats/**` at \
                 `public = true` with an \
                 open window requires a break note added in the SAME change \
                 (PROP-044 §4.7 — the gate does not forbid breaks, it makes an \
                 unannounced break impossible).\n\
                 why: foreign parsers read these bytes; a break they cannot \
                 see strands them in derived state before anyone knows.\n\
                 fix: write `formats/breaks/NNN.md` after the pattern of \
                 `formats/breaks/001.md` — what changed on the wire · epoch · \
                 who fixes it · sunset · user recipe — in the same change as \
                 the wire change, then re-run `cargo xtask wire-diff`."
            );
        }
        Verdict::Declared => {
            for line in shift_detail_lines(&shifted) {
                println!("{line}");
            }
            for note in &notes {
                println!("  wire-diff: declared by fresh note `{note}`");
            }
            println!(
                "wire-diff: green — the watched wire-surface shift is announced \
                 by a fresh \
                 break note (public = true, window open). The break is \
                 declared; the gate holds."
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;

    fn epochs(public: bool, break_window_open: bool) -> Epochs {
        Epochs {
            public,
            break_window_open,
        }
    }

    /// Row 1 — `public = false`, empty diff: green and quiet, at either
    /// window state.
    #[test]
    fn row1_pre_publication_and_clean_is_quiet() {
        assert_eq!(verdict(&epochs(false, false), false, false), Verdict::Quiet);
        assert_eq!(verdict(&epochs(false, true), false, true), Verdict::Quiet);
    }

    /// Row 2 — `public = false`, non-empty diff: green but REPORTING,
    /// at either window state, with or without a note.
    #[test]
    fn row2_pre_publication_and_shifted_is_reporting() {
        assert_eq!(
            verdict(&epochs(false, false), true, false),
            Verdict::Reporting
        );
        assert_eq!(
            verdict(&epochs(false, true), true, true),
            Verdict::Reporting
        );
    }

    /// Row 3 — `public = true`, closed window, non-empty diff: red,
    /// and a note cannot buy it out — the window rejects the change
    /// itself.
    #[test]
    fn row3_closed_window_rejects_even_with_a_note() {
        assert_eq!(
            verdict(&epochs(true, false), true, false),
            Verdict::ClosedWindow
        );
        assert_eq!(
            verdict(&epochs(true, false), true, true),
            Verdict::ClosedWindow
        );
    }

    /// Row 4 — `public = true`, open window, non-empty diff, no fresh
    /// note: red, the break is undeclared.
    #[test]
    fn row4_open_public_shift_without_a_note_is_undeclared() {
        assert_eq!(
            verdict(&epochs(true, true), true, false),
            Verdict::Undeclared
        );
    }

    /// Row 5 — `public = true`, open window, non-empty diff, fresh
    /// note: green.
    #[test]
    fn row5_open_public_shift_with_a_fresh_note_is_declared() {
        assert_eq!(verdict(&epochs(true, true), true, true), Verdict::Declared);
    }

    /// The empty cell the table leaves unstated: a closed window with a
    /// CLEAN corpus has nothing to reject — quiet, not red. The window
    /// governs changes; with no change there is no case.
    #[test]
    fn a_closed_window_with_a_clean_corpus_is_quiet() {
        assert_eq!(verdict(&epochs(true, false), false, false), Verdict::Quiet);
    }

    /// The path classifier is the operator-visible mirror of the two roots
    /// handed to the shift probe. It covers the old corpus surface, the newly
    /// watched schema surface, and format control files without admitting
    /// unrelated crate work.
    #[test]
    fn wire_shift_path_perimeter_is_schema_and_formats_only() {
        let cases = [
            ("formats/corpora/x", Some(ShiftClass::Corpus)),
            ("schemas/a/b.jtd.json", Some(ShiftClass::Schema)),
            ("formats/EPOCHS.toml", Some(ShiftClass::OtherFormat)),
            ("crates/example/src/lib.rs", None),
        ];

        for (path, expected) in cases {
            assert_eq!(shift_class(path), expected, "path: {path}");
        }
    }

    /// The diagnostic tail is derived from the same path list as its totals,
    /// including the repair-significant schema-without-corpus class.
    #[test]
    fn schema_only_shift_has_honest_distinct_output_class() {
        let shifted = vec!["schemas/hello/e1/hello.jtd.json".to_string()];
        let lines = shift_detail_lines(&shifted);

        assert_eq!(
            lines,
            vec![
                "wire-diff: shift classes — schema: 1, corpus: 0, other formats: 0",
                "wire-diff: schema changed without a corpus move — 1 schema path(s), 0 corpus path(s)",
                "  wire-diff: shifted `schemas/hello/e1/hello.jtd.json`",
            ]
        );
    }

    /// The «new note» test: untracked (`??`) and staged-add (`A…`)
    /// count; every status an OLD note can carry — clean (invisible),
    /// modified, or both — does not. This is the property that makes a
    /// stale note impossible to credit.
    #[test]
    fn only_untracked_and_staged_add_count_as_fresh_notes() {
        let fresh = |line: &str| {
            let (status, _) = porcelain_line(line).expect("a shaped porcelain line");
            status.contains('?') || status.starts_with('A')
        };
        assert!(fresh("?? formats/breaks/002.md"));
        assert!(fresh("A  formats/breaks/002.md"));
        assert!(fresh("AM formats/breaks/002.md"));
        assert!(!fresh("M  formats/breaks/001.md"));
        assert!(!fresh(" M formats/breaks/001.md"));
        assert!(!fresh("MM formats/breaks/001.md"));
        assert!(!fresh("D  formats/breaks/001.md"));
        assert!(!fresh("R  formats/breaks/old.md -> formats/breaks/001.md"));
    }

    /// A rename line credits the new path — the side added now.
    #[test]
    fn a_rename_line_credits_the_new_path() {
        assert_eq!(
            porcelain_line("R  spec/old.md -> formats/breaks/002.md"),
            Some(("R ", "formats/breaks/002.md"))
        );
        assert_eq!(
            porcelain_line("?? formats/breaks/002.md"),
            Some(("??", "formats/breaks/002.md"))
        );
        assert_eq!(porcelain_line("??"), None);
    }

    /// The REAL tree's registry names the index corpus — the
    /// denominator watchdog: wire-diff over zero corpora is a gate that
    /// cannot fail, and a registry edit that drops the `corpus` key
    /// makes `load_format_registry` refuse before silence can happen.
    #[test]
    fn the_real_tree_registry_names_the_index_corpus() -> Result<()> {
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set under cargo");
        let root = PathBuf::from(manifest_dir)
            .parent()
            .expect("xtask manifest dir has a parent")
            .to_path_buf();
        let dirs = corpus_dirs(&root)?;
        assert!(
            dirs.contains(&PathBuf::from("formats/corpora/index/e1")),
            "the registry must name the index corpus home, got: {dirs:?}"
        );
        assert!(
            dirs.iter().all(|d| root.join(d).is_dir()),
            "every named corpus home must exist in the tree, got: {dirs:?}"
        );
        Ok(())
    }
}
