//! Captured fact/adoption joins through existing owners, no terminal evaluator.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-READ-OBSERVATION-AXES");
use vibe_facts::{AuthoredFact, Registry, SourceKind};
use vibe_wire::generated::requirements_report::{RequirementRow, RequirementSourceKind};
pub fn change_fact_rows(
    registry: &Registry,
    coordinate: &str,
    package: bool,
    facts: &[AuthoredFact],
) -> Result<Vec<RequirementRow>, crate::QueryError> {
    let kind = if package {
        SourceKind::Package
    } else {
        SourceKind::Host
    };
    let joined = vibe_facts::join_adoption(registry, coordinate, kind, facts)
        .map_err(|error| crate::QueryError::Invariant(format!("change adoption join: {error}")))?;
    let wire_kind = if package {
        RequirementSourceKind::Package
    } else {
        RequirementSourceKind::Host
    };
    let rows = crate::rows::build(&wire_kind, coordinate, &joined);
    for row in &rows {
        vibe_wire::behaviour::requirements_report::validate_fact_metadata(row)
            .map_err(|source| crate::QueryError::Wire { source })?;
    }
    Ok(rows)
}

/// Optional existing local trace provider. Provenance/current-carried policy
/// remains in the incumbent resolver, separate from complete fact capture.
pub fn change_relations(
    rows: &mut [RequirementRow],
    requested: bool,
    provider: Option<&dyn crate::RelationProvider>,
    request: &crate::RelationRequest<'_>,
) -> Result<Vec<vibe_wire::generated::requirements_report::RelationSource>, crate::QueryError> {
    let resolved = crate::provider::resolve(requested, provider, request);
    for row in rows {
        if let Some(edges) = resolved.edges.get(&row.address) {
            row.relations = edges.clone();
        }
        vibe_wire::behaviour::requirements_report::validate_fact_metadata(row)
            .map_err(|source| crate::QueryError::Wire { source })?;
    }
    Ok(resolved.sources)
}
