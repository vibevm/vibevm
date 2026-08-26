//! The one generic artifact-row law, shared by reply acceptance and agent
//! preparation.
//!
//! Ordinary handlers are judged *after* they run, because a process either
//! produced rows or it did not. An agent execution is different: its rows are
//! fully determined by the declared contract **before** the provider is asked
//! for anything, so the same law can — and must — be applied before a token is
//! spent. Two copies of it would drift, and the copy that drifted would be the
//! pre-spend one, which is exactly the copy an operator relies on to refuse a
//! bad contract for free.
//!
//! The split is by evidence, not by caller: [`validate_shape`] is pure and
//! needs no filesystem, so it runs pre-spend; the physical checks (canonical
//! containment, link-free ancestry) stay with the post-write caller, because
//! before the write there is nothing to canonicalise.

use std::path::{Component, Path};

use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::Artifact;
use vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact;

/// The epoch-1 caps every artifact row obeys.
pub const MAX_ARTIFACTS: usize = 1024;
pub const MAX_ID_BYTES: usize = 256;
pub const MAX_KIND_BYTES: usize = 128;

/// Judge a complete set of planned or produced rows against the artifacts a
/// run has already accumulated. Pure: no path is opened, canonicalised or
/// created, so this is safe to run before any spend.
///
/// `project_root` is the envelope's spelling of the root — the same lexical
/// form every handler emits — and containment is judged lexically here. The
/// physical check belongs to whoever can actually see the file.
///
/// Path collisions are judged by **portable physical identity**
/// ([`vibe_safefs::path_identity_key`]), not by raw string equality: `Docs/A.md`
/// and `docs/a.md`, an NFC and an NFD spelling, `Maße` and `MASSE` are one file
/// on the volumes this ships to, and a raw comparison would let the second row
/// silently destroy the first. The stored spelling is untouched — identity is
/// only ever a comparison key.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub fn validate_shape(
    rows: &[ReplyArtifact],
    prior: &[Artifact],
    project_root: &str,
) -> Result<(), String> {
    if rows.len() > MAX_ARTIFACTS {
        return Err(format!(
            "{} artifact row(s) exceed the epoch-1 maximum of {MAX_ARTIFACTS}",
            rows.len(),
        ));
    }
    let root = project_root.trim_end_matches('/');
    let mut ids = std::collections::BTreeSet::new();
    let mut paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for row in rows {
        if row.id.is_empty() {
            return Err("an artifact id is empty".into());
        }
        if row.id.len() > MAX_ID_BYTES {
            return Err(format!(
                "artifact id `{}` is {} bytes; the epoch-1 maximum is {MAX_ID_BYTES}",
                truncated(&row.id),
                row.id.len(),
            ));
        }
        if row.kind.is_empty() || row.kind.len() > MAX_KIND_BYTES {
            return Err(format!(
                "artifact `{}` has an empty or over-long kind",
                truncated(&row.id)
            ));
        }
        check_path_shape(&row.id, &row.path, root)?;
        if !ids.insert(row.id.as_str()) {
            return Err(format!(
                "artifact id `{}` is declared more than once",
                truncated(&row.id)
            ));
        }
        let identity = vibe_safefs::path_identity_key(&row.path);
        if !paths.insert(identity.clone()) {
            return Err(format!(
                "artifact path `{}` names a file this set already declares; \
                 on a case-folding or normalising filesystem they are one file",
                truncated(&row.path)
            ));
        }
        if let Some(collision) = prior.iter().find(|earlier| {
            earlier.id == row.id || vibe_safefs::path_identity_key(&earlier.path) == identity
        }) {
            return Err(format!(
                "artifact `{}` collides with `{}`, already produced in phase `{}`",
                truncated(&row.id),
                truncated(&collision.id),
                collision.phase,
            ));
        }
    }
    Ok(())
}

/// The project-relative spelling of an artifact row's path, when that row is
/// one this project's capability may open.
///
/// Three answers, and the difference between them is what keeps the physical
/// gate honest:
///
/// - `Ok(Some(relative))` — the row is a well-formed path below this project,
///   so the OS can be asked whether a declared output is secretly the same
///   file;
/// - `Ok(None)` — the row is a well-formed **absolute** path that is genuinely
///   somewhere else. It is **skipped, never opened**: reaching outside the
///   project through the project capability is the exact thing the capability
///   exists to prevent, and a prior row is handler-supplied text;
/// - `Err(reason)` — the row is malformed. Refused, not skipped, and by the
///   same rules a planned row is judged by: an artifact we cannot locate is
///   one we cannot prove a declared output is not about to overwrite.
///
/// **Absoluteness is checked first, and that ordering is the point.** Artifact
/// rows are absolute by contract. A *relative* spelling — `docs/guide.md` —
/// shares no prefix with the root, so a prefix test alone would call it
/// "outside" and skip it silently. It is not outside; it is unlocatable, and
/// the file it means may be the very one a declared output is about to
/// overwrite. Skipping it would turn a corrupt record into a waived check.
///
/// A path exactly equal to the root is refused for the same reason: it names
/// no file below the project, so it is not an artifact row this law can judge.
///
/// The returned slice borrows the stored spelling; nothing here rewrites it.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub fn eligible_relative<'a>(
    id: &str,
    path: &'a str,
    project_root: &str,
) -> Result<Option<&'a str>, String> {
    let root = project_root.trim_end_matches('/');
    if path.contains('\\') {
        return Err(format!(
            "artifact `{}` has a backslash in its path",
            truncated(id)
        ));
    }
    if !Path::new(path).is_absolute() {
        return Err(format!(
            "artifact `{}` has a relative path; artifact rows are absolute, and a relative \
             one cannot be located well enough to rule out a collision",
            truncated(id)
        ));
    }
    let Some(rest) = path.strip_prefix(root) else {
        // Absolute and well-formed, and it shares no prefix with this project.
        return Ok(None);
    };
    let relative = match rest.strip_prefix('/') {
        Some(relative) => relative,
        // The root itself, with or without a trailing separator — not a file
        // below the project. (A non-empty `rest` without a leading `/` means
        // the root is merely a string prefix of a sibling directory's name, so
        // the row really is elsewhere.)
        None if rest.is_empty() => {
            return Err(format!(
                "artifact `{}` names the selected project root itself, not a file below it",
                truncated(id)
            ));
        }
        None => return Ok(None),
    };
    if relative.is_empty() {
        return Err(format!(
            "artifact `{}` names the selected project root itself, not a file below it",
            truncated(id)
        ));
    }
    if !Path::new(relative)
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "artifact `{}` has a non-normal path component",
            truncated(id)
        ));
    }
    Ok(Some(relative))
}

fn check_path_shape(id: &str, path: &str, root: &str) -> Result<(), String> {
    if path.contains('\\') {
        return Err(format!(
            "artifact `{}` has a backslash in its path",
            truncated(id)
        ));
    }
    if !Path::new(path).is_absolute() {
        return Err(format!(
            "artifact `{}` has a relative path; artifact rows are absolute",
            truncated(id)
        ));
    }
    let Some(relative) = path
        .strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return Err(format!(
            "artifact `{}` is not below the selected project",
            truncated(id)
        ));
    };
    if relative.is_empty()
        || !Path::new(relative)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "artifact `{}` has a non-normal path component",
            truncated(id)
        ));
    }
    Ok(())
}

/// Diagnostics quote handler-supplied text, so bound it.
fn truncated(value: &str) -> String {
    const LIMIT: usize = 120;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    format!(
        "{}… (truncated)",
        value.chars().take(LIMIT).collect::<String>()
    )
}

#[cfg(test)]
#[path = "artifacts/tests.rs"]
mod tests;
