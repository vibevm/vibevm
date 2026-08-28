//! The test-only legacy reference: HEAD's pre-R7.5 algorithm, verbatim — one
//! walk PER PATTERN, each match read at framing time. The byte-compatibility
//! REDs compare its end-to-end execution fingerprint against the one-walk
//! production path. It lives beside the code it mirrors (as a descendant of
//! the `fingerprint` module it reuses the UNCHANGED helpers: framing, lock
//! canonicalisation, validation), while the walk itself is deliberately the
//! old one.
use std::path::Path;

use glob::{MatchOptions, Pattern};
use vibe_wire::generated::lifecycle::e1::context::Context;
use walkdir::WalkDir;

use crate::ExtensionRegistryRow;
use crate::agent::PreparedAgent;

use super::{
    FingerprintError, FramedHash, file_or_missing, handler_coordinates, lockfile_identity,
    machine_path, provider_material, shippable_entry, validate_input,
};

pub(crate) fn execution_fingerprint_with(
    row: &ExtensionRegistryRow,
    context: &Context,
    prepared: Option<&PreparedAgent>,
) -> Result<String, FingerprintError> {
    let mut hash = FramedHash::new();
    hash.field("key", row.key().to_string().as_bytes());
    hash.field("phase", context.run.phase.as_bytes());
    hash.field("point", context.point.as_bytes());
    hash.json("slot-target", &context.slot_target, row.key())?;
    hash.field("handler-kind", row.declaration().handler.kind().as_bytes());
    handler_coordinates(&mut hash, &row.declaration().handler);
    hash.json("effective-config", &context.execution.config, row.key())?;
    provider_material(&mut hash, row.provider());
    hash.field("requested", context.run.requested.as_bytes());
    hash.json("chain", &context.run.chain, row.key())?;
    hash.field("offline", &[u8::from(context.run.offline)]);
    hash.json("agent-mode", &context.run.agent_mode, row.key())?;
    hash.json("project", &context.project, row.key())?;
    hash.json("world", &context.world, row.key())?;
    hash.json("artifacts", &context.artifacts, row.key())?;
    file_or_missing(
        &mut hash,
        "manifest",
        Path::new(&context.project.manifest),
        row.key(),
    )?;
    lockfile_identity(&mut hash, Path::new(&context.world.lockfile), row.key())?;
    declared_inputs_per_pattern(&mut hash, row, Path::new(&context.project.root))?;
    if let Some(prepared) = prepared {
        let (address, bytes) = prepared.fingerprint_material();
        hash.field("agent-prompt-address", address.as_bytes());
        hash.field("agent-prompt-bytes", bytes);
    }
    Ok(hash.finish())
}

/// HEAD's `declared_inputs`: for each authored pattern, frame it, walk
/// the whole tree for it alone, sort its matches, read and frame each.
fn declared_inputs_per_pattern(
    hash: &mut FramedHash,
    row: &ExtensionRegistryRow,
    project_root: &Path,
) -> Result<(), FingerprintError> {
    let Some(patterns) = row.declaration().inputs.as_deref() else {
        return Ok(());
    };
    for authored in patterns {
        validate_input(row.key(), authored)?;
        let pattern = Pattern::new(authored).map_err(|error| FingerprintError::InvalidInput {
            key: row.key().clone(),
            pattern: authored.clone(),
            reason: error.to_string(),
        })?;
        hash.field("input-pattern", authored.as_bytes());
        let mut matches = Vec::new();
        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_entry(shippable_entry)
        {
            let entry = entry.map_err(|error| FingerprintError::Read {
                key: row.key().clone(),
                path: machine_path(project_root),
                source: error
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walking input tree")),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative =
                entry
                    .path()
                    .strip_prefix(project_root)
                    .map_err(|_| FingerprintError::Read {
                        key: row.key().clone(),
                        path: machine_path(entry.path()),
                        source: std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "input walk escaped its project root",
                        ),
                    })?;
            let Some(relative) = relative.to_str() else {
                continue;
            };
            let relative = relative.replace('\\', "/");
            let options = MatchOptions {
                case_sensitive: true,
                require_literal_separator: true,
                require_literal_leading_dot: false,
            };
            if pattern.matches_with(&relative, options) {
                matches.push((relative, entry.into_path()));
            }
        }
        matches.sort_by(|left, right| left.0.cmp(&right.0));
        for (relative, path) in matches {
            let bytes = std::fs::read(&path).map_err(|source| FingerprintError::Read {
                key: row.key().clone(),
                path: machine_path(&path),
                source,
            })?;
            hash.field("input-path", relative.as_bytes());
            hash.field("input-bytes", &bytes);
        }
    }
    Ok(())
}
