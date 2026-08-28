//! The source universe: one discovery epoch, then typed per-source
//! results over the A2a one-read seams.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

use std::collections::BTreeMap;
use std::path::PathBuf;

use vibe_core::manifest::Lockfile;
use vibe_facts::{
    AuthoredFact, AuthoredSourceObservation, Registry, RegistryError, SourceKind as FactsKind,
    observe_authored_source, package_from_address,
};
use vibe_wire::generated::requirements_report::{
    RequirementSourceKind, SourceResult, SourceResultState,
};
use vibe_workspace::SelectedWorkspace;

use crate::QueryError;
use crate::digest::source_result_digest;

/// Fixed, bounded machine reasons — the wire's `reason_code` is one
/// bounded string per state, decided here so two runs of the same query
/// can never differ in wording.
pub(crate) const REASON_INVALID: &str = "authored-source-invalid";
pub(crate) const REASON_UNREADABLE: &str = "source-unreadable";
pub(crate) const REASON_NO_SLOT: &str = "no-materialised-slot";
pub(crate) const REASON_NO_SLOT_WITH_ENTRIES: &str = "no-materialised-slot-with-adoption-entries";
pub(crate) const REASON_REGISTRY_ONLY: &str = "registry-only-adoption-entries";

/// One enumerated source: its wire kind, coordinate and — when the
/// source physically exists here — the root the A2a scanner walks.
/// `locked` says the coordinate came from `vibe.lock` (its no-root
/// reason differs from a registry-only orphan's). `content_hash` is the
/// lock's exact authority for that package — present for EVERY
/// lock-selected coordinate (authority exists even when materialisation
/// does not), absent for the host and for registry-only orphans.
pub(crate) struct SourceCoord {
    pub kind: RequirementSourceKind,
    pub package: String,
    pub root: Option<PathBuf>,
    pub locked: bool,
    pub content_hash: Option<String>,
}

/// One source's contribution to the report: its typed result and, when
/// the source was read and parsed, the addressed facts it owns.
pub(crate) struct SourceOutcome {
    pub result: SourceResult,
    pub facts: Vec<AuthoredFact>,
}

/// The host coordinate from the one selected-workspace epoch — the
/// manifest of the node whose authored rel path IS `selected`, never a
/// second `vibe.toml` read. Error shapes and messages match what a
/// direct selected-manifest read produced, so surfacing is unchanged.
fn host_coordinate(selected: &SelectedWorkspace) -> Result<String, QueryError> {
    let manifest_path = selected.selected_root.join("vibe.toml");
    let invalid = |reason: &str| QueryError::Host {
        source: RegistryError::InvalidManifest {
            path: manifest_path.clone(),
            reason: reason.to_string(),
        },
    };
    let node = selected
        .workspace
        .iter_nodes()
        .find(|(rel, _)| *rel == selected.selected.as_str())
        .and_then(|(_, manifest)| manifest.consumer_node())
        .ok_or_else(|| invalid("the root manifest carries neither `[project]` nor `[package]`"))?;
    let Some(group) = &node.group else {
        return Err(invalid(
            "the root declares no `group` — spec:// addressing needs `<group>/<name>`",
        ));
    };
    Ok(format!("{group}/{}", node.name))
}

/// The adoption-entry count as the wire's `uint32` — an overflow is a
/// typed invariant error, never a saturated count pretending to be the
/// measurement.
pub(crate) fn checked_adoption_entries(count: u64) -> Result<u32, QueryError> {
    u32::try_from(count).map_err(|_| {
        QueryError::Invariant(format!(
            "adoption entry count {count} exceeds the wire's adoption_entries bound"
        ))
    })
}

/// Enumerate the source universe for one query: the selected host node
/// plus every lock-selected package coordinate with its slot, plus every
/// registry-only orphan coordinate. Sorted and unique by coordinate.
///
/// The host coordinate is derived from the SELECTED EPOCH's own node
/// manifest (`iter_nodes` matched on the authored `selected` rel path)
/// — no second `vibe.toml` read happens here. A missing lock yields no
/// package sources (the host still answers); a lock that is present but
/// not a regular file, or unreadable, is a typed query error. A
/// coordinate occurring under both kinds — or twice in the lock — is a
/// typed invariant error: the wire keys base sources by package alone
/// and must recover one kind.
pub(crate) fn enumerate(
    workspace_root: &std::path::Path,
    selected: &SelectedWorkspace,
    registry: &Registry,
) -> Result<Vec<SourceCoord>, QueryError> {
    let host = host_coordinate(selected)?;
    let selected_root = selected.selected_root.as_path();

    let mut coords: BTreeMap<String, SourceCoord> = BTreeMap::new();
    coords.insert(
        host.clone(),
        SourceCoord {
            kind: RequirementSourceKind::Host,
            package: host.clone(),
            root: Some(selected_root.to_path_buf()),
            locked: false,
            content_hash: None,
        },
    );

    // The lock is the package universe's authority. Absent ⇒ no
    // packages; present but not a readable file ⇒ the scope was never
    // established (a directory wearing the lock's name is malformed
    // scope, never a silent "no lock").
    let lock_path = workspace_root.join(Lockfile::FILENAME);
    if lock_path.exists() && !lock_path.is_file() {
        return Err(QueryError::LockNotFile {
            path: lock_path.clone(),
        });
    }
    if lock_path.is_file() {
        let lock = Lockfile::read(&lock_path).map_err(|source| QueryError::Lock { source })?;
        for locked in &lock.packages {
            let package = format!("{}/{}", locked.group, locked.name);
            if package == host {
                return Err(QueryError::Invariant(format!(
                    "coordinate `{package}` occurs as both the host and a locked package"
                )));
            }
            if coords.contains_key(&package) {
                return Err(QueryError::Invariant(format!(
                    "coordinate `{package}` occurs twice in the lock"
                )));
            }
            let slot = if locked.materialization.is_in_place() {
                vibe_workspace::vibedeps::in_place_slot_abs_path(
                    workspace_root,
                    &locked.group,
                    &locked.name,
                )
            } else {
                vibe_workspace::vibedeps::slot_abs_path(
                    workspace_root,
                    &locked.group,
                    &locked.name,
                    &locked.version,
                )
            };
            coords.insert(
                package.clone(),
                SourceCoord {
                    kind: RequirementSourceKind::Package,
                    package,
                    root: slot.is_dir().then_some(slot),
                    locked: true,
                    // The lock's exact authority for this package —
                    // kept even when the slot is absent (A2c: A3's
                    // carried-map trust needs it precisely for the
                    // missing/unmaterialised cases too).
                    content_hash: Some(locked.content_hash.to_string()),
                },
            );
        }
    }

    // Registry-only orphans: adoption entries addressed to a coordinate
    // this universe never enumerated. They become `orphaned` source
    // observations — never fake `unmarked` facts.
    let mut orphan_entries: BTreeMap<String, u64> = BTreeMap::new();
    for entry in registry.entries() {
        let coordinate =
            package_from_address(&entry.address).map_err(|source| QueryError::Registry {
                source: RegistryError::Invariant(format!(
                    "registry entry address did not parse: {source}"
                )),
            })?;
        if !coords.contains_key(&coordinate) {
            *orphan_entries.entry(coordinate).or_default() += 1;
        }
    }
    for package in orphan_entries.keys() {
        coords.insert(
            package.clone(),
            SourceCoord {
                kind: RequirementSourceKind::Package,
                package: package.clone(),
                root: None,
                locked: false,
                content_hash: None,
            },
        );
    }

    Ok(coords.into_values().collect())
}

/// The adoption-entry count for one coordinate, from the loaded
/// registry — used for the `orphaned` state's mandatory count.
pub(crate) fn adoption_entries_for(registry: &Registry, package: &str) -> u64 {
    registry
        .entries()
        .filter(|entry| {
            package_from_address(&entry.address)
                .map(|coordinate| coordinate == package)
                .unwrap_or(false)
        })
        .count() as u64
}

/// Observe one enumerated source through the A2a one-read seam and turn
/// the observation into its wire `SourceResult`.
///
/// `Available`/`Invalid` carry the recipe-1 digest over exactly the
/// documents the seam read; a raw-read/enumeration failure becomes
/// `unavailable` with the fixed unreadable reason (present bytes the
/// walk could not read are a typed loss, not a query abort); the
/// orphaned/unavailable slot rulings are decided by the caller from the
/// enumeration before this is called.
pub(crate) fn observe(coord: &SourceCoord) -> Result<SourceOutcome, QueryError> {
    let Some(root) = coord.root.as_deref() else {
        return Err(QueryError::Invariant(format!(
            "observing a source with no root: `{}`",
            coord.package
        )));
    };
    let kind = match coord.kind {
        RequirementSourceKind::Host => FactsKind::Host,
        RequirementSourceKind::Package => FactsKind::Package,
    };
    match observe_authored_source(root, &coord.package, kind) {
        Ok(AuthoredSourceObservation::Available { facts, documents }) => Ok(SourceOutcome {
            result: source_result(
                &coord.kind,
                &coord.package,
                SourceResultState::Available,
                Some(source_result_digest(
                    &coord.kind,
                    &coord.package,
                    &documents,
                )),
                None,
                None,
            ),
            facts,
        }),
        Ok(AuthoredSourceObservation::Invalid { documents, .. }) => Ok(SourceOutcome {
            result: source_result(
                &coord.kind,
                &coord.package,
                SourceResultState::Invalid,
                Some(source_result_digest(
                    &coord.kind,
                    &coord.package,
                    &documents,
                )),
                Some(REASON_INVALID.to_string()),
                None,
            ),
            facts: Vec::new(),
        }),
        Err(RegistryError::Io { .. }) => Ok(SourceOutcome {
            result: source_result(
                &coord.kind,
                &coord.package,
                SourceResultState::Unavailable,
                None,
                Some(REASON_UNREADABLE.to_string()),
                None,
            ),
            facts: Vec::new(),
        }),
        Err(source) => Err(QueryError::Invariant(format!(
            "unexpected scanner failure for `{}`: {source}",
            coord.package
        ))),
    }
}

/// The no-root ruling for one enumerated coordinate. A registry-only
/// coordinate (never locked) is `orphaned` by construction — its
/// entries ARE the observation. A lock-selected slot that is absent
/// here is `orphaned` when adoption entries survive it (the positive
/// count is the more informative observation, and the reason names both
/// facts), `unavailable` otherwise.
pub(crate) fn orphan_or_unavailable(
    coord: &SourceCoord,
    registry: &Registry,
) -> Result<SourceOutcome, QueryError> {
    if coord.kind != RequirementSourceKind::Package {
        return Err(QueryError::Invariant(format!(
            "a host source always has a root: `{}`",
            coord.package
        )));
    }
    let entries = adoption_entries_for(registry, &coord.package);
    let (state, reason, adoption_entries) = if !coord.locked {
        (
            SourceResultState::Orphaned,
            REASON_REGISTRY_ONLY.to_string(),
            Some(checked_adoption_entries(entries)?),
        )
    } else if entries > 0 {
        (
            SourceResultState::Orphaned,
            REASON_NO_SLOT_WITH_ENTRIES.to_string(),
            Some(checked_adoption_entries(entries)?),
        )
    } else {
        (
            SourceResultState::Unavailable,
            REASON_NO_SLOT.to_string(),
            None,
        )
    };
    Ok(SourceOutcome {
        result: source_result(
            &coord.kind,
            &coord.package,
            state,
            None,
            Some(reason),
            adoption_entries,
        ),
        facts: Vec::new(),
    })
}

/// Assemble one `SourceResult` in the wire's member shape.
pub(crate) fn source_result(
    kind: &RequirementSourceKind,
    package: &str,
    state: SourceResultState,
    digest: Option<String>,
    reason: Option<String>,
    adoption_entries: Option<u32>,
) -> SourceResult {
    SourceResult {
        source: vibe_wire::generated::requirements_report::RequirementSource {
            kind: kind.clone(),
            package: package.to_string(),
        },
        state,
        digest,
        reason_code: reason,
        adoption_entries,
    }
}
