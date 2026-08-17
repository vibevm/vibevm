//! The golden corpus of the index catalog — the proof that the catalog
//! is a *function of its journal*, and that the function has not
//! changed (PROP-044 §3 `MEMBERSHIP-IS-TESTED`, §4.3
//! `M-CANONICAL-BYTES`).
//!
//! The corpus at `formats/corpora/index/e1/` is two things: an
//! authored journal under `state/journal/` (the truth layer) and a
//! catalog committed beside it (the projection). This test reprojects
//! the journal into a temporary directory and byte-compares the result
//! against the committed catalog, file by file, `primary.jsonl.gz`
//! included. It is NOT a test that "the catalog looks like this": the
//! committed bytes carry no authority of their own. It is a test that
//! the projection — replay → fold → `write_to` — still maps this
//! journal to these bytes. When it fails, either the journal or the
//! projection code changed; the failure names the file and the first
//! diverging line so the reader can tell which.
//!
//! What the corpus covers, and why it is shaped the way it is: three
//! packages and five standing versions, carrying every dictionary of
//! the catalog's wire (including one package kind and one delivery
//! mode unknown to this build — the open vocabularies of PROP-044
//! §4.2a), every per-version slot both filled and empty, every
//! optional projection both present and absent, a short-name
//! collision across groups, a prerelease that loses `latest_stable`
//! to an older stable, and all six projectable journal event
//! variants. The one slot the corpus cannot carry — a name-level
//! tombstone — is a measured gap, not an oversight: no projectable
//! journal event produces one, so a catalog born by projection cannot
//! contain one without breaking the very contract this test guards.
//!
//! Offline by construction: the journal is the only input, the
//! projection is pure, and no network or registry is touched.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_index::index::memory::WriteCtx;
use vibe_index::index::{WRITER_DIRS, WRITER_FILES};
use vibe_index::journal;

/// The corpus root: the journal lives under `state/journal/` inside
/// it, the catalog files beside that — the layout `journal::default_dir`
/// and `cargo xtask rebuild --check` already dictate, restated here so
/// the test can find the fixture from the crate directory.
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("formats")
        .join("corpora")
        .join("index")
        .join("e1")
}

/// Project the corpus's journal into `target_dir`, exactly the way
/// `xtask rebuild --check` does: replay, fold, write — the clock taken
/// from the fold itself, never from this machine. Returns nothing; the
/// assertion is the caller's byte-compare against the committed catalog.
fn project_corpus_into(target_dir: &Path) {
    let corpus = corpus_dir();
    assert!(
        journal::default_dir(&corpus).is_dir(),
        "the corpus journal is missing at `{}`",
        journal::default_dir(&corpus).display()
    );
    let records =
        journal::replay(&journal::default_dir(&corpus)).expect("the corpus journal replays");
    let index = journal::project(records).expect("the corpus journal folds");
    let at = index.generated_at;
    index
        .write_to(target_dir, &WriteCtx { at })
        .expect("the corpus projection writes");
}

/// Every file under `root` (recursively), relative to it.
fn file_tree(root: &Path) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("walking `{}`: {e}", dir.display()))
            .map(|e| e.expect("a directory entry reads"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .expect("path under its root")
                        .to_path_buf(),
                );
            }
        }
    }
    files
}

/// The committed corpus's writer surface — the fixed files that exist,
/// plus everything under the three writer directories.
fn corpus_surface(corpus: &Path) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    for name in WRITER_FILES {
        if corpus.join(name).is_file() {
            files.insert(PathBuf::from(name));
        }
    }
    for dir in WRITER_DIRS {
        let root = corpus.join(dir);
        if root.is_dir() {
            for rel in file_tree(&root) {
                files.insert(Path::new(dir).join(rel));
            }
        }
    }
    files
}

/// The first diverging line of two byte sequences, as `(line number,
/// committed line, projected line)` — `None` when the bytes are equal.
/// Line numbers are 1-based, matching how a text editor counts them.
fn first_diverging_line(committed: &[u8], projected: &[u8]) -> Option<(usize, String, String)> {
    let committed_lines = split_lines(committed);
    let projected_lines = split_lines(projected);
    for (index, (a, b)) in committed_lines
        .iter()
        .zip(projected_lines.iter())
        .enumerate()
    {
        if a != b {
            return Some((
                index + 1,
                String::from_utf8_lossy(a).into_owned(),
                String::from_utf8_lossy(b).into_owned(),
            ));
        }
    }
    // Every common line agreed, so the divergence is where one side
    // ends: name the first line the shorter side does not have.
    if committed_lines.len() != projected_lines.len() {
        let index = committed_lines.len().min(projected_lines.len());
        let line_at = |lines: &[&[u8]], i: usize| {
            lines
                .get(i)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_else(|| "«this side ends here»".to_string())
        };
        return Some((
            index + 1,
            line_at(&committed_lines, index),
            line_at(&projected_lines, index),
        ));
    }
    None
}

/// Split into lines (without terminators) the way a diff does.
fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            lines.push(&bytes[start..i]);
            start = i + 1;
        }
    }
    if start < bytes.len() {
        lines.push(&bytes[start..]);
    }
    lines
}

/// The committed catalog is byte-identical to the projection of the
/// committed journal — file for file, gzip envelope included. A failure
/// names the file, and for text files the first diverging line.
#[test]
fn the_catalog_is_the_projection_of_its_journal() {
    let corpus = corpus_dir();
    let scratch = TempDir::new().expect("a scratch directory");
    let projection_dir = scratch.path().join("projection");
    project_corpus_into(&projection_dir);

    let projected = file_tree(&projection_dir);
    let committed = corpus_surface(&corpus);

    let mut findings: Vec<String> = Vec::new();
    for rel in projected.union(&committed) {
        let in_projection = projected.contains(rel);
        let in_corpus = committed.contains(rel);
        match (in_projection, in_corpus) {
            (true, false) => findings.push(format!(
                "`{}`: the projection writes this file, the committed corpus lacks it",
                rel.display()
            )),
            (false, true) => findings.push(format!(
                "`{}`: the committed corpus holds this file, the projection does not produce it",
                rel.display()
            )),
            (true, true) => {
                let projected_bytes = fs::read(projection_dir.join(rel))
                    .unwrap_or_else(|e| panic!("reading projection `{}`: {e}", rel.display()));
                let committed_bytes = fs::read(corpus.join(rel))
                    .unwrap_or_else(|e| panic!("reading corpus `{}`: {e}", rel.display()));
                if projected_bytes == committed_bytes {
                    continue;
                }
                let is_gzip = rel.extension().is_some_and(|e| e == "gz");
                let is_text = !is_gzip
                    && std::str::from_utf8(&committed_bytes).is_ok()
                    && std::str::from_utf8(&projected_bytes).is_ok();
                if is_text {
                    if let Some((lineno, was, now)) =
                        first_diverging_line(&committed_bytes, &projected_bytes)
                    {
                        findings.push(format!(
                            "`{}` differs first at line {lineno}:\n  committed: {was}\n  projected: {now}",
                            rel.display()
                        ));
                    }
                } else {
                    findings.push(format!(
                        "`{}` differs (binary): committed {} byte(s), projected {} byte(s)",
                        rel.display(),
                        committed_bytes.len(),
                        projected_bytes.len()
                    ));
                }
            }
            (false, false) => unreachable!("the union holds members of at least one set"),
        }
    }
    assert!(
        findings.is_empty(),
        "the golden corpus no longer reproduces from its journal — either the \
         journal or the projection changed:\n  {}",
        findings.join("\n  ")
    );
}
