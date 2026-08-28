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
//!
//! A4b wraps the byte source in certified stable observation
//! ([`super::stable`]): accepted bytes feed BOTH projections; an
//! evidence-only refusal falls back to one ordinary raw read that feeds the
//! legacy fingerprint alone and refuses the WHOLE manifest. One `WalkDir`
//! enumeration and one logical union row per selected regular path remain
//! the law — one physical OS read never was (the two-read detection law
//! needs two).

use std::path::Path;

use glob::{MatchOptions, Pattern};
use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::{Context, SlotTarget};
use vibe_wire::generated::lifecycle_state::{StateDigestWitness, StateInputMeasurement};
use walkdir::WalkDir;

use vibe_core::manifest::ExtensionKey;

use crate::agent::PreparedAgent;
use crate::{ExtensionRegistryRow, HandlerExecution};

use super::stable::{InputRefusal, observe};
use super::{
    FingerprintError, FramedHash, declaration, file_or_missing, handler_coordinates,
    lockfile_identity, machine_path, provider_material, shippable_entry, validate_input,
};

/// The manifest algorithm name, also the prefix of its domain seed.
const MANIFEST_ALGORITHM: &str = "sha256:vibe-input-manifest-v1";

/// The domain-separated seed every input manifest starts from — a second
/// hasher, never the execution fingerprint's.
const MANIFEST_SEED: &[u8] = b"sha256:vibe-input-manifest-v1\0epoch=1\0";

/// One execution fingerprint, its declaration sibling and the declared-input
/// manifest the SAME single walk produced. `input_manifest` is `None`
/// exactly when the declaration is absent (`inputs = null`): an absent
/// declaration is `unavailable`, never an empty-set digest in disguise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedFingerprint {
    pub(crate) fingerprint: String,
    /// The effective executable declaration's own identity
    /// (PROP-054 `##DECLARATION-FINGERPRINT`) — the sibling the durable
    /// measurement names, never an alias of `fingerprint`.
    pub(crate) declaration_fingerprint: String,
    pub(crate) input_manifest: Option<PreparedInputManifest>,
}

/// The deduplicated half of the one walk: the declaration-order patterns and
/// the outcome of witnessing the union they select. `Refused` keeps the
/// patterns (the declared scope is a fact) but witnesses nothing — a
/// refused scope produces no partial or subset measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedInputManifest {
    pub(crate) patterns: Vec<String>,
    pub(crate) outcome: ManifestOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManifestOutcome {
    Measured(StateDigestWitness),
    Refused(InputRefusal),
}

impl PreparedInputManifest {
    /// The measured witness, or `None` when the observation was refused.
    pub(crate) fn measured(&self) -> Option<&StateDigestWitness> {
        match &self.outcome {
            ManifestOutcome::Measured(witness) => Some(witness),
            ManifestOutcome::Refused(_) => None,
        }
    }
}

impl PreparedFingerprint {
    /// The durable state row this observation owes: present exactly when the
    /// declared scope was measured, naming the execution, phase, declaration
    /// fingerprint, patterns, the CURRENT run id and the witness. A refused
    /// or absent scope yields `None` — never a partial measurement, never a
    /// copied prior claim.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-MEASUREMENT-CARRIAGE"
    )]
    pub(crate) fn state_measurement(
        &self,
        execution: &str,
        phase: &str,
        run_id: &str,
    ) -> Option<StateInputMeasurement> {
        let manifest = self.input_manifest.as_ref()?;
        let witness = manifest.measured()?;
        Some(StateInputMeasurement {
            declaration_fingerprint: self.declaration_fingerprint.clone(),
            execution: execution.to_string(),
            measured_run_id: run_id.to_string(),
            patterns: manifest.patterns.clone(),
            phase: phase.to_string(),
            witness: witness.clone(),
        })
    }
}

/// The handler-level entry the runner consumes: the row fingerprint, the
/// slot-target wrapping that only a targeted row owes, and the input manifest
/// (which slot wrapping never touches — a manifest is a tree observation, not
/// a per-slot identity). The declaration fingerprint is computed from the
/// EXACT `HandlerExecution::key()` and descriptor slot coordinate.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub(crate) fn prepare_handler_execution_with(
    execution: &HandlerExecution,
    context: &Context,
    prepared_agent: Option<&PreparedAgent>,
) -> Result<PreparedFingerprint, FingerprintError> {
    let mut prepared = prepare_with_identity(
        execution.row(),
        &execution.key(),
        execution.slot_target(),
        context,
        prepared_agent,
    )?;
    if execution.slot_target().is_some() {
        let mut hash = FramedHash::new();
        hash.field("base-fingerprint", prepared.fingerprint.as_bytes());
        hash.field("execution-identity", execution.key().as_bytes());
        prepared.fingerprint = hash.finish();
    }
    Ok(prepared)
}

/// The row-level sibling the public `fingerprint_execution_with` delegates
/// to, so delegation does not have to invent a `HandlerExecution`. Its
/// declaration digest uses the bare row key — the legacy public row API
/// discards it, so no caller may treat it as a slot-qualified identity.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub(crate) fn prepare_execution_with(
    row: &ExtensionRegistryRow,
    context: &Context,
    prepared_agent: Option<&PreparedAgent>,
) -> Result<PreparedFingerprint, FingerprintError> {
    prepare_with_identity(
        row,
        &row.key().to_string(),
        context.slot_target.as_ref(),
        context,
        prepared_agent,
    )
}

fn prepare_with_identity(
    row: &ExtensionRegistryRow,
    execution_identity: &str,
    slot: Option<&SlotTarget>,
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
    let input_manifest = inputs
        .as_ref()
        .map(|inputs| inputs.manifest(row.key()))
        .transpose()?;
    let declaration_fingerprint = declaration::declaration_fingerprint(
        execution_identity,
        slot,
        row,
        &context.run.phase,
        &context.execution.config,
        prepared_agent,
    )?;
    Ok(PreparedFingerprint {
        fingerprint: hash.finish(),
        declaration_fingerprint,
        input_manifest,
    })
}

/// Everything the one walk collected: the union rows sorted by canonical
/// forward-slashed relative path, per pattern (declaration order) the
/// indexes of the union rows it selects — already in path order because the
/// union is sorted and indexes are pushed in union order — and the first
/// evidence refusal the walk observed, if any.
struct PreparedInputs {
    patterns: Vec<String>,
    files: Vec<UnionFile>,
    per_pattern: Vec<Vec<usize>>,
    refusal: Option<InputRefusal>,
}

struct UnionFile {
    relative: String,
    bytes: Vec<u8>,
}

impl PreparedInputs {
    /// Replay the LEGACY pattern-major stream into the execution
    /// fingerprint's hasher, byte-for-byte what the pre-R7.5 per-pattern walk
    /// framed — overlapping and duplicate patterns intentionally repeat a
    /// file exactly as HEAD did. Refused paths contribute their ordinary raw
    /// fallback bytes exactly as HEAD's single read did.
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

    /// The deduplicated projection: the manifest outcome over patterns plus
    /// the union, under its own SHA-256 domain. A refusal witnesses nothing.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-SCOPE-IS-DECLARED")]
    fn manifest(&self, key: &ExtensionKey) -> Result<PreparedInputManifest, FingerprintError> {
        if let Some(cause) = self.refusal {
            return Ok(PreparedInputManifest {
                patterns: self.patterns.clone(),
                outcome: ManifestOutcome::Refused(cause),
            });
        }
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
            outcome: ManifestOutcome::Measured(StateDigestWitness {
                algorithm: MANIFEST_ALGORITHM.to_string(),
                digest: format!("sha256:{:x}", hash.finalize()),
                files: Some(checked_file_count(self.files.len(), key)?),
                bytes: Some(total_bytes.to_string()),
            }),
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
/// the walk, then walk the project root exactly once. Patterns are matched
/// BEFORE non-files are discarded, so a SELECTED link/reparse/device refuses
/// the whole evidence manifest while the legacy projection — which never
/// read such an entry — keeps ignoring it; a selected DIRECTORY is merely
/// not a file and stays ignored. Each selected regular path is observed
/// through the certified two-read protocol; an evidence-only refusal takes
/// one ordinary raw read for the legacy stream alone.
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
            refusal: None,
        }));
    }
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    // Opening the evidence capability is not allowed to veto the legacy
    // execution decision. If it refuses, each selected regular path below
    // takes the old raw-read path and the manifest is refused; a pattern that
    // selects no files can still witness its real empty set.
    let project = vibe_safefs::Project::open(project_root).ok();
    #[cfg(test)]
    observe::count_walk();
    let mut hits = Vec::new();
    let mut refusal: Option<InputRefusal> = None;
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
        let file_type = entry.file_type();
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
        if file_type.is_dir() {
            // An ordinary selected directory is not an input file. A Windows
            // junction/reparse can present directory-shaped metadata, so the
            // shared no-follow walk must distinguish it before this branch is
            // ignored. This is refusal evidence only; neither form is read by
            // the legacy projection.
            if vibe_safefs::ensure_no_follow_walk(project_root, entry.path(), false).is_err() {
                refusal = refusal.or(Some(InputRefusal::NonRegular));
            }
            continue;
        }
        if !file_type.is_file() {
            // A SELECTED link, junction, reparse point or device. The legacy
            // scanner read nothing here and must never follow it; its
            // selection alone refuses the whole evidence manifest.
            refusal = refusal.or(Some(InputRefusal::NonRegular));
            continue;
        }
        let path = entry.into_path();
        let bytes = match project
            .as_ref()
            .ok_or(InputRefusal::Open)
            .and_then(|project| observe(project, &relative))
        {
            Ok(certified) => certified,
            Err(cause) => {
                // Evidence-only refusal on a legacy-regular path: ONE
                // ordinary raw read still feeds the legacy fingerprint —
                // HEAD's exact bytes, hardlinks included — and the whole
                // manifest is refused. Enabling evidence never changes
                // freshness and never vetoes the handler.
                #[cfg(test)]
                observe::count_raw_fallback();
                refusal = refusal.or(Some(cause));
                std::fs::read(&path).map_err(|source| FingerprintError::Read {
                    key: row.key().clone(),
                    path: machine_path(&path),
                    source,
                })?
            }
        };
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
    // The shared portable identity judgment over the regular union: two
    // selected spellings that fold to one physical file refuse the manifest
    // after both legacy rows have retained their old bytes.
    if refusal.is_none()
        && vibe_safefs::judge_selection(files.iter().map(|file| file.relative.as_str())).is_err()
    {
        refusal = Some(InputRefusal::Aliased);
    }
    Ok(Some(PreparedInputs {
        patterns: patterns.to_vec(),
        files,
        per_pattern,
        refusal,
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
/// The two-read law retired the old per-file raw-read count: what is pinned
/// now is ONE walk per preparation and ZERO raw fallbacks on a clean tree
/// (fallbacks appear exactly once per refused legacy-regular path).
#[cfg(test)]
pub(crate) mod observe {
    use std::cell::Cell;

    thread_local! {
        static WALKS: Cell<usize> = const { Cell::new(0) };
        static RAW_FALLBACKS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        WALKS.with(|walks| walks.set(0));
        RAW_FALLBACKS.with(|fallbacks| fallbacks.set(0));
    }

    pub(crate) fn walks() -> usize {
        WALKS.with(Cell::get)
    }

    pub(crate) fn raw_fallbacks() -> usize {
        RAW_FALLBACKS.with(Cell::get)
    }

    pub(super) fn count_walk() {
        WALKS.with(|walks| walks.set(walks.get() + 1));
    }

    pub(super) fn count_raw_fallback() {
        RAW_FALLBACKS.with(|fallbacks| fallbacks.set(fallbacks.get() + 1));
    }
}
