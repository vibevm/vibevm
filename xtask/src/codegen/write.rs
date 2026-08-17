//! Writing a generated file on a filesystem where something else may
//! hold the previous bytes mapped.
//!
//! The law this obeys is the project's own, and it predates this
//! module: **never overwrite a file that may be in use — write a new
//! instance and flip the pointer**
//! (`spec://org.vibevm.world/tool-design-lessons/…#never`). For a file
//! the pointer is its name, so a fresh sibling takes the content and
//! then takes the name.
//!
//! What made a rule that had been dormant for the whole life of this
//! layer suddenly load-bearing: the shared-module step writes ONE
//! generated file up to three times in a single run — the generator
//! emits it, the post-processing passes rewrite it, and the replacement
//! rewrites it again. On Windows a file that was written a moment ago is
//! routinely open in another process's mapped section (an indexer, a
//! scanner), and a plain overwrite of it fails with
//! `os error 1224 — the requested operation cannot be performed on a
//! file with a user-mapped section open`. Measured here 2026-08-17:
//! three consecutive runs, each failing on a DIFFERENT module — so it is
//! a race against a foreign reader, not one stuck file, and tripling the
//! writes per run is what widened the window enough to hit it reliably.
//!
//! Why a rename survives where a write does not: the mapped section
//! keeps the file's CONTENT alive, while the rename only rewrites the
//! directory entry — the loser of the race ends up holding bytes that no
//! name points at, which is exactly the outcome "a new instance plus a
//! pointer flip" is supposed to produce. A retry loop was rejected: it
//! would make the layer's correctness depend on how long a foreign
//! process happens to hold a section, and a gate whose verdict depends
//! on someone else's timing is not a gate.

use std::path::Path;

use anyhow::{Context, Result};

/// Write `content` to `path` by way of a sibling temporary, then rename
/// it over the target. The temporary lives beside its target rather than
/// in the system temp dir, because a rename across volumes is a copy —
/// and a copy is the very overwrite this function exists to avoid.
///
/// The temporary's name is derived from the target's, so two writers of
/// DIFFERENT files never collide, and a crash leaves a `.tmp` beside the
/// file it was going to become rather than an unnamed remnant somewhere
/// else. Codegen wipes its output tree before every run, so such a
/// remnant cannot outlive one run.
pub(crate) fn write_generated(path: &Path, content: &str) -> Result<()> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        // A path with no final component cannot be a file we generated.
        anyhow::bail!(
            "{}: not a file path — a generated file is always written to \
             a named leaf, so this is a defect in the caller rather than \
             in the filesystem.",
            path.display()
        );
    };
    let temporary = path.with_file_name(format!("{name}.tmp"));
    std::fs::write(&temporary, content)
        .with_context(|| format!("writing the new instance {}", temporary.display()))?;
    std::fs::rename(&temporary, path).with_context(|| {
        format!(
            "renaming {} over {} — the pointer flip that replaces the \
             file without overwriting bytes another process may hold \
             mapped",
            temporary.display(),
            path.display()
        )
    })
}
