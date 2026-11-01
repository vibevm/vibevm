//! `vibe cache import <dir>` — place a transport bundle's artifacts
//! (`PROP-066#transport`).
//!
//! The counterpart of `vibe cache add` for a network that cannot reach the
//! sources: instead of fetching, it reads a bundle written by
//! `vibe remap export --with-artifacts`, verifies every digest, and places
//! each entry where the fetch paths already look — the machine package
//! store by coordinate, the application download cache by digest. It never
//! overwrites an entry that already verifies: the store is write-once, and
//! arriving on a disk is no reason to make it otherwise.
//!
//! The verb lives in the cache family rather than under `remap` because
//! what it writes is the cache, and `PROP-010 §2.8`'s surface is where an
//! operator looks for "put these bytes in the store".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#transport");

use anyhow::Result;

use crate::cli::CacheImportArgs;
use crate::commands::remap::bundle;
use crate::output;

pub(crate) fn run(ctx: &output::Context, args: CacheImportArgs) -> Result<()> {
    let dir = args.dir.canonicalize().unwrap_or_else(|_| args.dir.clone());
    let document = bundle::read_manifest(&dir)?;
    let task = ctx.progress().task("Importing bundled artifacts");
    let imported = match bundle::place(&dir, &document) {
        Ok(imported) => {
            task.set_progress(
                (imported.placed.len() + imported.already.len()) as u64,
                Some(document.artifacts.len() as u64),
                "artifacts",
            );
            task.finish();
            imported
        }
        Err(error) => {
            task.fail(error.to_string());
            return Err(error);
        }
    };
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "cache:import",
            "dir": dir.display().to_string(),
            "package": document.package,
            "version": document.version,
            "placed": imported.placed,
            "already_present": imported.already,
            "refused": imported.refused
                .iter()
                .map(|(what, why)| serde_json::json!({ "artifact": what, "reason": why }))
                .collect::<Vec<_>>(),
            "count": imported.placed.len() + imported.already.len(),
        }));
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe cache import: {} placed, {} already present, {} refused",
            imported.placed.len(),
            imported.already.len(),
            imported.refused.len()
        ));
        return Ok(());
    }
    ctx.rule(&format!("importing {}", dir.display()));
    for label in &imported.placed {
        ctx.created(label);
    }
    for label in &imported.already {
        ctx.skipped(label, "already present and verified — bytes untouched");
    }
    // A refused entry is said out loud rather than counted: a digest that
    // does not match is the one thing a bundle must never place quietly.
    for (label, reason) in &imported.refused {
        ctx.diagnostic(&format!("{label} not placed — {reason}"));
    }
    ctx.trailer(&format!(
        "imported {} · {} placed · {} already present · {} refused",
        dir.display(),
        imported.placed.len(),
        imported.already.len(),
        imported.refused.len()
    ));
    Ok(())
}
