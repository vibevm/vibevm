//! Out-of-line `#[cfg(test)] mod` bodies — the cross-file half of test
//! scoping the per-file [`Extractor`](crate::Extractor) cannot see.
//!
//! A body-less `#[cfg(test)] mod tests;` declaration puts the module body in
//! a sibling file (`tests.rs` / `tests/mod.rs`, or a `#[path = "…"]`-named
//! one). `syn` parses the declaration with `content: None`, so the body file
//! is scanned INDEPENDENTLY and its facts land `in_test: false` — the
//! V8-OUTOFLINE-TESTS defect: an out-of-line test file reads as domain.
//!
//! The body's test status is a property of the *declaration* (in the parent
//! file), not of the body itself, so it cannot be established inside the
//! per-file [`Frontend::extract`](conform_core::Frontend::extract) — that
//! sees one file at a time. It is established cross-file, after the whole
//! workspace is extracted: re-scan every file for `#[cfg(test)] mod <name>;`
//! declarations, resolve each to its body path(s) by Rust's module rules,
//! and stamp the body files' facts `in_test`. The shared `Fact` model lives
//! in a vendored crate this package builds against (not editable here), so
//! the signal is carried by re-derivation, not a new fact variant — see the
//! recorded limit on caching.
//!
//! **Safety (the load-bearing property):** a file is marked test ONLY because
//! a real `#[cfg(test)]` attribute on a body-less `mod` points at it through
//! Rust's resolution rules — never by name, never by content. A production
//! `mod foo;` (no `cfg(test)`) contributes nothing, so its body's `unwrap`s
//! stay findings. The candidate body path is matched against the files the
//! store actually scanned, so a dangling declaration marks nothing.

specmark::scope!(
    "spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#frontends"
);

use std::collections::HashSet;
use std::path::Path;

use conform_core::{Fact, SourceFacts};

use crate::is_cfg_test;

/// The directory a `mod <name>;` declaration in `file` resolves its body
/// against, by Rust's 2018 module rules: `mod.rs` / `lib.rs` / `main.rs`
/// look in their OWN directory; any other `<stem>.rs` looks in a sibling
/// directory NAMED after the stem. `file` is the repo-relative, forward-
/// slashed path the engine scans.
pub(crate) fn mod_dir_of(file: &str) -> String {
    let (dir, base) = match file.rsplit_once('/') {
        Some((d, b)) => (d, b),
        None => ("", file),
    };
    let stem = base.strip_suffix(".rs").unwrap_or(base);
    match stem {
        "lib" | "main" | "mod" => dir.to_string(),
        stem if dir.is_empty() => stem.to_string(),
        stem => format!("{dir}/{stem}"),
    }
}

/// Join `dir` and `rel` as a forward-slashed path, resolving `.` and `..` —
/// the shape repo-relative scan paths take, so a `#[path]` value like
/// `lib/tests.rs` (or a rare `../x.rs`) lands on the path the store scans.
/// `..` past the root is dropped (a malformed path); the function never
/// panics.
fn join_forward(dir: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').filter(|s| !s.is_empty()).collect()
    };
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

/// The string literal of a `#[path = "…"]` attribute on a module, if any —
/// the explicit override of a body's location (relative to the declaring
/// file's directory). The same structural shape as `cfg` detection: a `path`
/// ident whose meta is a string-literal name-value.
fn path_attr_value(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|a| {
        if !a.path().is_ident("path") {
            return None;
        }
        let syn::Meta::NameValue(nv) = &a.meta else {
            return None;
        };
        match &nv.value {
            syn::Expr::Lit(lit) => {
                if let syn::Lit::Str(s) = &lit.lit {
                    Some(s.value())
                } else {
                    None
                }
            }
            _ => None,
        }
    })
}

/// The candidate body-file paths a body-less `#[cfg(test)] mod` declaration
/// in `file` points at, by Rust's resolution rules or an explicit `#[path]`.
/// Top-level declarations only (see the module's recorded limit on nesting).
/// Returns BOTH the `<name>.rs` and `<name>/mod.rs` forms when no `#[path]`
/// pins one: extraction cannot see the filesystem to choose, and the caller
/// stamps only the form the store actually scanned.
pub(crate) fn out_line_test_body_paths(file: &str, text: &str) -> Vec<String> {
    let Ok(ast) = syn::parse_file(text) else {
        return Vec::new();
    };
    let dir = file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut out = Vec::new();
    for item in &ast.items {
        let syn::Item::Mod(m) = item else {
            continue;
        };
        // An INLINE `#[cfg(test)] mod tests { … }` carries its own body —
        // the per-file Extractor's depth logic already scopes it, and there
        // is no separate file to mark. Only a body-less declaration points
        // at a sibling file.
        if m.content.is_some() || !is_cfg_test(&m.attrs) {
            continue;
        }
        let name = m.ident.to_string();
        match path_attr_value(&m.attrs) {
            // A `#[path]` ending in `.rs` names one file outright.
            Some(p) if p.ends_with(".rs") => out.push(join_forward(dir, &p)),
            // A `#[path]` without `.rs` may name a file or a directory.
            Some(p) => {
                let base = join_forward(dir, &p);
                out.push(format!("{base}.rs"));
                out.push(format!("{base}/mod.rs"));
            }
            // No `#[path]`: the two forms Rust's resolver tries.
            None => {
                let mdir = mod_dir_of(file);
                out.push(format!("{mdir}/{name}.rs"));
                out.push(format!("{mdir}/{name}/mod.rs"));
            }
        }
    }
    out
}

/// The set of repo-relative files that are the BODY of some out-of-line
/// `#[cfg(test)] mod` declaration, across a whole workspace's
/// `(repo-relative path, source text)` pairs. Pure (no filesystem): the
/// caller supplies what it scanned, so this is unit-testable in memory.
///
/// A file is in the set ONLY because a real declaration under `cfg(test)`
/// points at it through Rust's resolution rules — never by name, never by
/// content. A plain `mod foo;` (no `cfg(test)`) contributes nothing, and a
/// file the store did not scan is never matched (see [`stamp_test_context`]).
pub fn out_of_line_test_bodies(files: &[(String, String)]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|(file, text)| out_line_test_body_paths(file, text))
        .collect()
}

/// Stamp `in_test = true` on every Rust fact in a file the caller established
/// is an out-of-line `#[cfg(test)]` body — the four `in_test`-carrying facts
/// the rules scope out in test context (`UnwrapUse`, `UnsafeUse`, `EnvRead`,
/// `InvariantComment`). Mutation, not re-extraction: the body's raw facts are
/// already extracted; this corrects the one field the per-file scan could not
/// know. Idempotent (a second call is a no-op).
pub fn stamp_test_context(facts: &mut [SourceFacts], bodies: &HashSet<String>) {
    for sf in facts {
        if !bodies.contains(&sf.file) {
            continue;
        }
        for fact in &mut sf.facts {
            match fact {
                Fact::UnwrapUse { in_test, .. }
                | Fact::UnsafeUse { in_test, .. }
                | Fact::EnvRead { in_test, .. }
                | Fact::InvariantComment { in_test, .. } => *in_test = true,
                _ => {}
            }
        }
    }
}

/// Establish out-of-line test context over a whole extracted workspace:
/// re-read each scanned file, find every out-of-line `#[cfg(test)] mod`
/// declaration, and stamp the resulting body files' facts `in_test`. The one
/// call a conform driver makes after
/// [`Store::extract_workspace`](conform_core::Store::extract_workspace) — the
/// body's test status is cross-file, so it is applied here, once, over the
/// full fact set (cached or freshly extracted alike), not during the per-file
/// extract that cannot see it.
///
/// Re-reads the source (the store's own read is internal to extraction); a
/// file that cannot be read is skipped, matching the store's tolerance. The
/// stamp is a runtime correction over collected facts — never written back to
/// the content-addressed cache — so a body cached `in_test: false` is
/// corrected every run from the *current* declaration (cache-safe by
/// construction; see the module's recorded limit).
pub fn apply_out_of_line_test_context(repo: &Path, facts: &mut [SourceFacts]) {
    let files: Vec<(String, String)> = facts
        .iter()
        .filter_map(|sf| {
            let text = std::fs::read_to_string(repo.join(&sf.file)).ok()?;
            Some((sf.file.clone(), text))
        })
        .collect();
    let bodies = out_of_line_test_bodies(&files);
    stamp_test_context(facts, &bodies);
}
