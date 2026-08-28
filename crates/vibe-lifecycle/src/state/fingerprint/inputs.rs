//! The one-walk declared-input preparation — two projections of ONE walk.
//!
//! PROP-054 `##EVIDENCE-SCOPE-IS-DECLARED` and the R7.5 architecture §4.1
//! split the old per-pattern walk into a single physical walk that serves two
//! deliberate projections. The legacy execution fingerprint replays its
//! pattern-major stream byte-for-byte — each authored pattern in declaration
//! order, then every matching path/byte in path order, INCLUDING the
//! historical repeat when patterns overlap — while the evidence manifest
//! hashes the deduplicated union so one physical path contributes one file
//! and one byte count. An absent `inputs` declaration produces no measurement
//! at all; an authored empty list is a complete empty scope with a real
//! zero-count digest under the `sha256:vibe-input-manifest-v1` domain.

use std::path::Path;

use glob::{MatchOptions, Pattern};
use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::Context;
use vibe_wire::generated::lifecycle_state::StateDigestWitness;
use walkdir::WalkDir;

use vibe_core::manifest::ExtensionKey;

use crate::agent::PreparedAgent;
use crate::{ExtensionRegistryRow, HandlerExecution};

use super::{
    FingerprintError, FramedHash, file_or_missing, handler_coordinates, lockfile_identity,
    machine_path, provider_material, shippable_entry, validate_input,
};

/// The manifest algorithm name, also the prefix of its domain seed.
const MANIFEST_ALGORITHM: &str = "sha256:vibe-input-manifest-v1";

/// The domain-separated seed every input manifest starts from — a second
/// hasher, never the execution fingerprint's.
const MANIFEST_SEED: &[u8] = b"sha256:vibe-input-manifest-v1\0epoch=1\0";

/// One execution fingerprint plus the declared-input manifest the SAME single
/// walk produced. `input_manifest` is `None` exactly when the declaration is
/// absent (`inputs = null`): an absent declaration is `unavailable`, never an
/// empty-set digest in disguise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedFingerprint {
    pub(crate) fingerprint: String,
    pub(crate) input_manifest: Option<PreparedInputManifest>,
}

/// The deduplicated half of the one walk: the declaration-order patterns and
/// one content witness over the union they select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedInputManifest {
    pub(crate) patterns: Vec<String>,
    pub(crate) witness: StateDigestWitness,
}

/// The handler-level entry A4b's runner consumes: the row fingerprint, the
/// slot-target wrapping that only a targeted row owes, and the input manifest
/// (which slot wrapping never touches — a manifest is a tree observation, not
/// a per-slot identity).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub(crate) fn prepare_handler_execution_with(
    execution: &HandlerExecution,
    context: &Context,
    prepared_agent: Option<&PreparedAgent>,
) -> Result<PreparedFingerprint, FingerprintError> {
    let mut prepared = prepare_execution_with(execution.row(), context, prepared_agent)?;
    if execution.slot_target().is_some() {
        let mut hash = FramedHash::new();
        hash.field("base-fingerprint", prepared.fingerprint.as_bytes());
        hash.field("execution-identity", execution.key().as_bytes());
        prepared.fingerprint = hash.finish();
    }
    Ok(prepared)
}

/// The row-level sibling the public `fingerprint_execution_with` delegates
/// to, so delegation does not have to invent a `HandlerExecution`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub(crate) fn prepare_execution_with(
    row: &ExtensionRegistryRow,
    context: &Context,
    prepared_agent: Option<&PreparedAgent>,
) -> Result<PreparedFingerprint, FingerprintError> {
    let inputs = prepare_declared_inputs(row, Path::new(&context.project.root))?;
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
    if let Some(inputs) = inputs.as_ref() {
        inputs.fold_legacy_into(&mut hash);
    }
    if let Some(prepared) = prepared_agent {
        let (address, bytes) = prepared.fingerprint_material();
        hash.field("agent-prompt-address", address.as_bytes());
        hash.field("agent-prompt-bytes", bytes);
    }
    let input_manifest = match inputs {
        Some(inputs) => Some(inputs.manifest(row.key())?),
        None => None,
    };
    Ok(PreparedFingerprint {
        fingerprint: hash.finish(),
        input_manifest,
    })
}

/// Everything the one walk collected: the union rows sorted by canonical
/// forward-slashed relative path, and per pattern (declaration order) the
/// indexes of the union rows it selects — already in path order because the
/// union is sorted and indexes are pushed in union order.
struct PreparedInputs {
    patterns: Vec<String>,
    files: Vec<UnionFile>,
    per_pattern: Vec<Vec<usize>>,
}

struct UnionFile {
    relative: String,
    bytes: Vec<u8>,
}

impl PreparedInputs {
    /// Replay the LEGACY pattern-major stream into the execution
    /// fingerprint's hasher, byte-for-byte what the pre-R7.5 per-pattern walk
    /// framed — overlapping and duplicate patterns intentionally repeat a
    /// file exactly as HEAD did.
    fn fold_legacy_into(&self, hash: &mut FramedHash) {
        for (authored, matches) in self.patterns.iter().zip(&self.per_pattern) {
            hash.field("input-pattern", authored.as_bytes());
            for &index in matches {
                let file = &self.files[index];
                hash.field("input-path", file.relative.as_bytes());
                hash.field("input-bytes", &file.bytes);
            }
        }
    }

    /// The deduplicated projection: the manifest witness over patterns plus
    /// the union, under its own SHA-256 domain.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-SCOPE-IS-DECLARED")]
    fn manifest(&self, key: &ExtensionKey) -> Result<PreparedInputManifest, FingerprintError> {
        let mut hash = Sha256::new();
        hash.update(MANIFEST_SEED);
        frame(&mut hash, "pattern_count", self.patterns.len().to_string());
        for pattern in &self.patterns {
            frame(&mut hash, "pattern", pattern.as_bytes());
        }
        frame(&mut hash, "file_count", self.files.len().to_string());
        let sizes = self.files.iter().map(|file| file.bytes.len() as u128);
        let total_bytes = checked_total_bytes(sizes, key)?;
        for file in &self.files {
            frame(&mut hash, "path", file.relative.as_bytes());
            frame(&mut hash, "size", file.bytes.len().to_string());
            frame(&mut hash, "bytes", &file.bytes);
        }
        frame(&mut hash, "total_bytes", total_bytes.to_string());
        Ok(PreparedInputManifest {
            patterns: self.patterns.clone(),
            witness: StateDigestWitness {
                algorithm: MANIFEST_ALGORITHM.to_string(),
                digest: format!("sha256:{:x}", hash.finalize()),
                files: Some(checked_file_count(self.files.len(), key)?),
                bytes: Some(total_bytes.to_string()),
            },
        })
    }
}

/// The existing field frame `be64(label_len)||label||be64(value_len)||value`,
/// applied to the manifest's own hasher.
fn frame(hash: &mut Sha256, label: &str, value: impl AsRef<[u8]>) {
    let value = value.as_ref();
    hash.update((label.len() as u64).to_be_bytes());
    hash.update(label.as_bytes());
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

/// `files` narrows to the witness wire's `u32` ONLY through this checked
/// conversion — saturation would mint a false witness over a silently
/// truncated count.
pub(crate) fn checked_file_count(
    files: usize,
    key: &ExtensionKey,
) -> Result<u32, FingerprintError> {
    u32::try_from(files).map_err(|_| FingerprintError::InputManifestOverflow {
        key: key.clone(),
        reason: format!(
            "the declared-input union selects {files} files, beyond the witness `files` bound"
        ),
    })
}

/// The `total_bytes` accumulator never narrows: each size enters as `u128`
/// and every addition is checked, so the count can never wrap.
pub(crate) fn checked_total_bytes<I>(sizes: I, key: &ExtensionKey) -> Result<u128, FingerprintError>
where
    I: IntoIterator<Item: Into<u128>>,
{
    let mut total = 0_u128;
    for size in sizes {
        total = total.checked_add(size.into()).ok_or_else(|| {
            FingerprintError::InputManifestOverflow {
                key: key.clone(),
                reason: "the declared-input union's total byte count overflows".to_string(),
            }
        })?;
    }
    Ok(total)
}

/// Compile and validate EVERY authored pattern in declaration order BEFORE
/// the walk, then walk the project root exactly once, reading each file no
/// pattern selects never, and each file one or many patterns select exactly
/// once.
fn prepare_declared_inputs(
    row: &ExtensionRegistryRow,
    project_root: &Path,
) -> Result<Option<PreparedInputs>, FingerprintError> {
    let Some(patterns) = row.declaration().inputs.as_deref() else {
        return Ok(None);
    };
    let mut compiled = Vec::with_capacity(patterns.len());
    for authored in patterns {
        validate_input(row.key(), authored)?;
        let pattern = Pattern::new(authored).map_err(|error| FingerprintError::InvalidInput {
            key: row.key().clone(),
            pattern: authored.clone(),
            reason: error.to_string(),
        })?;
        compiled.push(pattern);
    }
    if compiled.is_empty() {
        // An authored empty list is a complete empty declared scope: no tree
        // walk, but a real empty manifest and witness.
        return Ok(Some(PreparedInputs {
            patterns: Vec::new(),
            files: Vec::new(),
            per_pattern: Vec::new(),
        }));
    }
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    #[cfg(test)]
    observe::count_walk();
    let mut hits = Vec::new();
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
        // Authored patterns are UTF-8. A non-UTF-8 filesystem name is
        // outside that namespace and cannot be selected; skipping it
        // avoids both false failures and lossy alias/order collisions.
        let Some(relative) = relative.to_str() else {
            continue;
        };
        let relative = relative.replace('\\', "/");
        let ordinals: Vec<usize> = compiled
            .iter()
            .enumerate()
            .filter(|(_, pattern)| pattern.matches_with(&relative, options))
            .map(|(ordinal, _)| ordinal)
            .collect();
        if ordinals.is_empty() {
            // No pattern selects it: never read it.
            continue;
        }
        let path = entry.into_path();
        #[cfg(test)]
        observe::count_read();
        let bytes = std::fs::read(&path).map_err(|source| FingerprintError::Read {
            key: row.key().clone(),
            path: machine_path(&path),
            source,
        })?;
        hits.push(Hit {
            relative,
            ordinals,
            bytes,
        });
    }
    hits.sort_by(|left, right| left.relative.cmp(&right.relative));
    let mut per_pattern = vec![Vec::new(); compiled.len()];
    let mut files = Vec::with_capacity(hits.len());
    for (index, hit) in hits.into_iter().enumerate() {
        for ordinal in hit.ordinals {
            per_pattern[ordinal].push(index);
        }
        files.push(UnionFile {
            relative: hit.relative,
            bytes: hit.bytes,
        });
    }
    Ok(Some(PreparedInputs {
        patterns: patterns.to_vec(),
        files,
        per_pattern,
    }))
}

struct Hit {
    relative: String,
    ordinals: Vec<usize>,
    bytes: Vec<u8>,
}

/// Test-only observers for the one-walk law. Thread-local for the same
/// reason the state-write fault seams are: parallel test threads must not
/// count each other's walks, and a release build compiles them out entirely.
#[cfg(test)]
pub(crate) mod observe {
    use std::cell::Cell;

    thread_local! {
        static WALKS: Cell<usize> = const { Cell::new(0) };
        static READS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        WALKS.with(|walks| walks.set(0));
        READS.with(|reads| reads.set(0));
    }

    pub(crate) fn walks() -> usize {
        WALKS.with(Cell::get)
    }

    pub(crate) fn reads() -> usize {
        READS.with(Cell::get)
    }

    pub(super) fn count_walk() {
        WALKS.with(|walks| walks.set(walks.get() + 1));
    }

    pub(super) fn count_read() {
        READS.with(|reads| reads.set(reads.get() + 1));
    }
}

/// The test-only legacy reference: HEAD's pre-R7.5 algorithm, verbatim — one
/// walk PER PATTERN, each match read at framing time. The byte-compatibility
/// REDs compare its end-to-end execution fingerprint against the one-walk
/// production path. It lives beside the code it mirrors (as a descendant of
/// the `fingerprint` module it reuses the UNCHANGED helpers: framing, lock
/// canonicalisation, validation), while the walk itself is deliberately the
/// old one.
#[cfg(test)]
pub(crate) mod legacy {
    use std::path::Path;

    use glob::{MatchOptions, Pattern};
    use vibe_wire::generated::lifecycle::e1::context::Context;
    use walkdir::WalkDir;

    use crate::ExtensionRegistryRow;
    use crate::agent::PreparedAgent;

    use super::super::{
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
            let pattern =
                Pattern::new(authored).map_err(|error| FingerprintError::InvalidInput {
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
                let relative = entry.path().strip_prefix(project_root).map_err(|_| {
                    FingerprintError::Read {
                        key: row.key().clone(),
                        path: machine_path(entry.path()),
                        source: std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "input walk escaped its project root",
                        ),
                    }
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
}
