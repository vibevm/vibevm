//! The two source layers and the coherence between them.
//!
//! `sources[]` is the BASE layer: whether an authored source could be
//! read at all. `relation_sources[]` is the ENRICHMENT layer: what
//! optional relation data it could offer. Keeping them apart is the
//! whole correction — a malformed authored source is not «relations
//! unavailable», and a registry-only orphan is a source observation
//! rather than a fabricated `unmarked` fact row
//! (`##FACT-QUERY-CONTRACT`).
//!
//! This file owns the two matrices and the cross-layer laws; the entry
//! and the per-row laws stay in the parent.

use std::collections::BTreeMap;

use crate::generated::requirements_report::{
    RelationSource, RelationSourceProvenance, RelationSourceState, RequirementSourceKind,
    RequirementsReport, SourceResult, SourceResultState,
};

use super::errors::{ReasonDefect, RequirementsError, SourceDefect, SourceStateDefect};
use super::{bounded, coordinates::is_coordinate, preview};

/// One base source result, indexed by its package. The entry carries
/// the KIND because `relation_sources[]` is keyed by package alone and
/// its fresh-vs-carried law has to read the kind back off this layer.
#[derive(Debug, Clone)]
pub(super) struct SourceEntry<'a> {
    pub(super) state: SourceResultState,
    pub(super) kind: &'a RequirementSourceKind,
}

/// package → the source result. The identity key is the PACKAGE, not
/// `(kind, package)`: `relation_sources` names a package and nothing
/// else, so two base results for one coordinate under different kinds
/// would make the provenance verdict depend on which one a lookup
/// happened to reach first. Making the sort key and the identity key
/// the same string is what turns «sorted» and «unique» into one check.
pub(super) type SourceIndex<'a> = BTreeMap<&'a str, SourceEntry<'a>>;

/// `source-coherence` + `source-result-matrix` over the base layer:
/// parseable coordinates, results sorted and unique BY PACKAGE, and
/// the four-state member matrix.
pub(super) fn sources_gate(
    report: &RequirementsReport,
) -> Result<SourceIndex<'_>, RequirementsError> {
    let mut index: SourceIndex<'_> = BTreeMap::new();
    let mut previous: Option<&str> = None;
    for (position, result) in report.sources.iter().enumerate() {
        bounded(&result.source.package, "sources.package")?;
        if let Some(reason) = result.reason_code.as_deref() {
            bounded(reason, "sources.reason_code")?;
        }
        if !is_coordinate(&result.source.package) {
            return Err(RequirementsError::SourceCoherence {
                index: position,
                package: preview(&result.source.package),
                defect: SourceDefect::NotACoordinate,
            });
        }
        let key = result.source.package.as_str();
        if let Some(previous) = previous {
            let defect = match previous.cmp(key) {
                std::cmp::Ordering::Less => None,
                std::cmp::Ordering::Equal => Some(SourceDefect::DuplicatePackage),
                std::cmp::Ordering::Greater => Some(SourceDefect::OutOfOrder),
            };
            if let Some(defect) = defect {
                return Err(RequirementsError::SourceCoherence {
                    index: position,
                    package: preview(&result.source.package),
                    defect,
                });
            }
        }
        previous = Some(key);
        source_matrix(position, result)?;
        index.insert(
            key,
            SourceEntry {
                state: result.state.clone(),
                kind: &result.source.kind,
            },
        );
    }
    Ok(index)
}

/// `source-result-matrix`: the digest, the reason and the adoption
/// count are each present exactly where the state means something by
/// them.
fn source_matrix(index: usize, result: &SourceResult) -> Result<(), RequirementsError> {
    use SourceResultState as S;
    let (wants_digest, wants_reason, wants_entries) = match result.state {
        S::Available => (true, false, false),
        S::Unavailable => (false, true, false),
        S::Invalid => (true, true, false),
        S::Orphaned => (false, true, true),
    };
    let refuse = |defect: SourceStateDefect| {
        Err(RequirementsError::SourceStateMatrix {
            index,
            state: result.state.clone(),
            defect,
        })
    };
    if let Some(digest) = result.digest.as_deref() {
        bounded(digest, "sources.digest")?;
        if !wants_digest {
            return refuse(SourceStateDefect::UnexpectedDigest);
        }
        if !super::is_sha256(digest) {
            return refuse(SourceStateDefect::DigestShape);
        }
    } else if wants_digest {
        return refuse(SourceStateDefect::AbsentDigest);
    }
    match result.reason_code.as_deref() {
        Some(reason) if reason.trim().is_empty() => {
            return Err(RequirementsError::SourceReason {
                index,
                state: result.state.clone(),
                defect: ReasonDefect::Blank,
            });
        }
        Some(_) if !wants_reason => {
            return Err(RequirementsError::SourceReason {
                index,
                state: result.state.clone(),
                defect: ReasonDefect::Unexpected,
            });
        }
        None if wants_reason => {
            return Err(RequirementsError::SourceReason {
                index,
                state: result.state.clone(),
                defect: ReasonDefect::Absent,
            });
        }
        _ => {}
    }
    match result.adoption_entries {
        Some(0) if wants_entries => refuse(SourceStateDefect::ZeroAdoptionEntries),
        Some(_) if !wants_entries => refuse(SourceStateDefect::UnexpectedAdoptionEntries),
        None if wants_entries => refuse(SourceStateDefect::AbsentAdoptionEntries),
        _ => Ok(()),
    }
}

/// `source-coherence` + `relation-state-matrix` over the enrichment
/// layer: parseable coordinates, sorted unique states, the
/// state↔provenance↔reason matrix, and the kind constraint the base
/// layer supplies.
pub(super) fn relation_sources_gate(
    report: &RequirementsReport,
    sources: &SourceIndex<'_>,
) -> Result<(), RequirementsError> {
    let mut previous: Option<&str> = None;
    for (index, source) in report.relation_sources.iter().enumerate() {
        bounded(&source.package, "relation_sources.package")?;
        if let Some(reason) = source.reason_code.as_deref() {
            bounded(reason, "relation_sources.reason_code")?;
        }
        if !is_coordinate(&source.package) {
            return Err(RequirementsError::SourceCoherence {
                index,
                package: preview(&source.package),
                defect: SourceDefect::NotACoordinate,
            });
        }
        if let Some(previous) = previous {
            let defect = match previous.cmp(source.package.as_str()) {
                std::cmp::Ordering::Less => None,
                std::cmp::Ordering::Equal => Some(SourceDefect::DuplicatePackage),
                std::cmp::Ordering::Greater => Some(SourceDefect::OutOfOrder),
            };
            if let Some(defect) = defect {
                return Err(RequirementsError::SourceCoherence {
                    index,
                    package: preview(&source.package),
                    defect,
                });
            }
        }
        previous = Some(&source.package);
        relation_matrix(index, source, report.query.relations)?;
        provenance_kind(index, source, sources)?;
    }
    Ok(())
}

/// One relation source's own matrix clause.
fn relation_matrix(
    index: usize,
    source: &RelationSource,
    requested: bool,
) -> Result<(), RequirementsError> {
    use RelationSourceProvenance as P;
    use RelationSourceState as S;
    let expected: &[P] = match source.state {
        S::NotRequested | S::Unavailable => &[P::None],
        S::Current => &[P::FreshProjectMap],
        S::Carried => &[P::CarriedPackageMap],
        S::Stale | S::Invalid => &[P::FreshProjectMap, P::CarriedPackageMap],
    };
    if !expected.contains(&source.provenance) {
        return Err(RequirementsError::RelationProvenance {
            index,
            state: source.state.clone(),
            provenance: source.provenance.clone(),
        });
    }
    let owes_reason = matches!(source.state, S::Stale | S::Unavailable | S::Invalid);
    match source.reason_code.as_deref() {
        Some(reason) if reason.trim().is_empty() => Err(RequirementsError::RelationReason {
            index,
            state: source.state.clone(),
            defect: ReasonDefect::Blank,
        }),
        Some(_) if !owes_reason => Err(RequirementsError::RelationReason {
            index,
            state: source.state.clone(),
            defect: ReasonDefect::Unexpected,
        }),
        None if owes_reason => Err(RequirementsError::RelationReason {
            index,
            state: source.state.clone(),
            defect: ReasonDefect::Absent,
        }),
        _ => {
            let not_requested = matches!(source.state, S::NotRequested);
            if not_requested == requested {
                Err(RequirementsError::RelationRequestMismatch {
                    index,
                    state: source.state.clone(),
                    requested,
                })
            } else {
                Ok(())
            }
        }
    }
}

/// The kind constraint: a HOST package's relation data is built fresh
/// in memory and an installed PACKAGE's is carried in its slot, so the
/// provenance a state may pair with is decided by the base layer's
/// kind, not by the writer.
///
/// The lookup is TOTAL. `relation_sources` names a package and nothing
/// else, so a package the base layer never named would leave this law
/// with no kind to apply — and skipping it silently is how a document
/// acquires a verdict nobody checked. An enrichment state for a source
/// this answer never enumerated is refused as incoherent instead.
fn provenance_kind(
    index: usize,
    source: &RelationSource,
    sources: &SourceIndex<'_>,
) -> Result<(), RequirementsError> {
    use RelationSourceProvenance as P;
    let Some(kind) = sources.get(source.package.as_str()).map(|entry| entry.kind) else {
        return Err(RequirementsError::SourceCoherence {
            index,
            package: preview(&source.package),
            defect: SourceDefect::NoBaseSource,
        });
    };
    let forbidden = match kind {
        RequirementSourceKind::Host => P::CarriedPackageMap,
        RequirementSourceKind::Package => P::FreshProjectMap,
    };
    if source.provenance == forbidden {
        return Err(RequirementsError::RelationProvenanceKind {
            index,
            package: preview(&source.package),
            kind: kind.clone(),
            provenance: source.provenance.clone(),
        });
    }
    let forbidden_state = match kind {
        RequirementSourceKind::Host => RelationSourceState::Carried,
        RequirementSourceKind::Package => RelationSourceState::Current,
    };
    if source.state == forbidden_state {
        return Err(RequirementsError::RelationProvenanceKind {
            index,
            package: preview(&source.package),
            kind: kind.clone(),
            provenance: source.provenance.clone(),
        });
    }
    Ok(())
}

/// The cross-layer half of `relation-state-matrix`: what edges may
/// exist at all, and which packages must be answered for.
pub(super) fn relation_coverage(report: &RequirementsReport) -> Result<(), RequirementsError> {
    let named: BTreeMap<&str, ()> = report
        .relation_sources
        .iter()
        .map(|source| (source.package.as_str(), ()))
        .collect();
    for (index, row) in report.rows.iter().enumerate() {
        if !report.query.relations {
            if row.relations.is_empty() {
                continue;
            }
            return Err(RequirementsError::EdgesWithoutRequest { index });
        }
        // Relations WERE requested, so every package that owns a row is
        // answered for — even with zero edges. Silence about a package
        // that was scanned is not an answer.
        if !named.contains_key(row.source.package.as_str()) {
            return Err(RequirementsError::RelationSourceMissing {
                index,
                package: preview(&row.source.package),
            });
        }
    }
    Ok(())
}
