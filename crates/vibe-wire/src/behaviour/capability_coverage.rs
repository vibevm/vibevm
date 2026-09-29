//! Pure coverage declarations; validation grants no source or runtime proof.

use std::collections::{BTreeMap, BTreeSet};

use crate::generated::capability::e1::coverage::{Coverage, Disposition, Surface, SurfaceKind};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#COVERAGE-INVENTORY");

/// A declaration defect, located by field and the opaque owning row identity.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("capability coverage {field} ({subject}): {reason}")]
pub struct CapabilityCoverageError {
    pub field: &'static str,
    pub subject: String,
    pub reason: &'static str,
}

fn fault(field: &'static str, subject: &str, reason: &'static str) -> CapabilityCoverageError {
    CapabilityCoverageError {
        field,
        subject: if subject.trim().is_empty() {
            "inventory row".into()
        } else {
            subject.into()
        },
        reason,
    }
}

fn nonblank(
    field: &'static str,
    subject: &str,
    value: &str,
) -> Result<(), CapabilityCoverageError> {
    if value.trim().is_empty() {
        return Err(fault(field, subject, "required declaration is blank"));
    }
    Ok(())
}

fn unique_strings(
    values: &[String],
    field: &'static str,
    subject: &str,
) -> Result<(), CapabilityCoverageError> {
    let mut seen = BTreeSet::new();
    for value in values {
        nonblank(field, subject, value)?;
        if !seen.insert(value.as_str()) {
            return Err(fault(field, subject, "repeated declaration"));
        }
    }
    Ok(())
}

fn source_refs(
    values: &[String],
    sources: &BTreeSet<&str>,
    field: &'static str,
    subject: &str,
) -> Result<(), CapabilityCoverageError> {
    if values.is_empty() {
        return Err(fault(
            field,
            subject,
            "at least one source reference is required",
        ));
    }
    unique_strings(values, field, subject)?;
    if values.iter().any(|value| !sources.contains(value.as_str())) {
        return Err(fault(field, subject, "source reference does not exist"));
    }
    Ok(())
}

fn source_path(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.contains(['\0', '\\'])
        && value
            .split('/')
            .all(|part| !matches!(part, "" | "." | "..") && !part.contains(':'))
}

fn source_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn kind_key(kind: &SurfaceKind) -> u8 {
    match kind {
        SurfaceKind::Cli => 0,
        SurfaceKind::Mcp => 1,
        SurfaceKind::Tui => 2,
        SurfaceKind::ExtensionRegistration => 3,
        SurfaceKind::DeveloperTool => 4,
    }
}

fn surface_fields(
    surface: &Surface,
    sources: &BTreeSet<&str>,
) -> Result<(), CapabilityCoverageError> {
    let id = surface.surface_id.as_str();
    for (field, value) in [
        ("surfaces.surface_id", id),
        ("surfaces.name", surface.name.as_str()),
        ("surfaces.owner", &surface.owner),
        ("surfaces.target", &surface.target),
        ("surfaces.authority", &surface.authority),
        ("surfaces.carrier", &surface.carrier),
        ("surfaces.schema_ref", &surface.schema_ref),
        ("surfaces.completeness", &surface.completeness),
        ("surfaces.retention", &surface.retention),
        ("surfaces.reason", &surface.reason),
    ] {
        nonblank(field, id, value)?;
    }
    let registered_role = match surface.kind {
        SurfaceKind::Cli | SurfaceKind::Mcp | SurfaceKind::DeveloperTool => surface.registered,
        SurfaceKind::ExtensionRegistration => !surface.registered,
        SurfaceKind::Tui => true,
    };
    if !registered_role {
        return Err(fault(
            "surfaces.registered",
            id,
            "registration role contradicts surface kind",
        ));
    }
    for (field, value) in [
        (
            "surfaces.parent_surface_id",
            surface.parent_surface_id.as_deref(),
        ),
        ("surfaces.operation_id", surface.operation_id.as_deref()),
    ] {
        if let Some(value) = value {
            nonblank(field, id, value)?;
        }
    }
    if surface.disposition == Disposition::Operation && surface.operation_id.is_none() {
        return Err(fault(
            "surfaces.operation_id",
            id,
            "operation disposition requires an operation identity",
        ));
    }
    unique_strings(&surface.aliases, "surfaces.aliases", id)?;
    unique_strings(&surface.effects, "surfaces.effects", id)?;
    unique_strings(&surface.conformance_cases, "surfaces.conformance_cases", id)?;
    source_refs(&surface.source_ids, sources, "surfaces.source_ids", id)
}

fn surface_relations(surfaces: &BTreeMap<&str, &Surface>) -> Result<(), CapabilityCoverageError> {
    let mut tokens = BTreeSet::new();
    for (&id, surface) in surfaces {
        if let Some(parent_id) = surface.parent_surface_id.as_deref() {
            let parent = surfaces.get(parent_id).ok_or_else(|| {
                fault(
                    "surfaces.parent_surface_id",
                    id,
                    "parent surface does not exist",
                )
            })?;
            if parent_id == id || parent.kind != surface.kind {
                return Err(fault(
                    "surfaces.parent_surface_id",
                    id,
                    "parent must be a different surface of the same kind",
                ));
            }
        }
        for token in std::iter::once(&surface.name).chain(&surface.aliases) {
            let key = (
                kind_key(&surface.kind),
                surface.parent_surface_id.as_deref(),
                token.as_str(),
            );
            if !tokens.insert(key) {
                return Err(fault(
                    "surfaces.names",
                    id,
                    "canonical name or alias collides within one kind and parent",
                ));
            }
        }
    }
    // Parent edges form a forest: walk iteratively, retaining completed chains.
    // No recursion or repeated traversal of an already checked ancestor is needed.
    let mut complete = BTreeSet::new();
    for &id in surfaces.keys() {
        let mut active = BTreeSet::new();
        let mut current = Some(id);
        while let Some(at) = current {
            if complete.contains(at) {
                break;
            }
            if !active.insert(at) {
                return Err(fault("surfaces.parent_surface_id", id, "parent cycle"));
            }
            current = surfaces[at].parent_surface_id.as_deref();
        }
        complete.extend(active);
    }
    Ok(())
}

impl Coverage {
    /// Validate declaration and reference consistency without reading any source.
    ///
    /// IDs remain opaque and validation does not change the inventory. Empty effect
    /// and conformance lists remain declarations awaiting independent review.
    /// Success proves neither actual registration coverage nor authorization,
    /// implementation, source freshness, callable semantics or executed evidence.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#COVERAGE-INVENTORY"
    )]
    pub fn validate_capability_coverage(&self) -> Result<(), CapabilityCoverageError> {
        let inventory = self.inventory_id.as_str();
        if self.schema != 1 {
            return Err(fault("schema", inventory, "unsupported epoch"));
        }
        for (field, value) in [
            ("inventory_id", inventory),
            ("baseline_capture_id", self.baseline_capture_id.as_str()),
            (
                "observation_capture_id",
                self.observation_capture_id.as_str(),
            ),
        ] {
            nonblank(field, inventory, value)?;
        }
        for (field, empty) in [
            ("sources", self.sources.is_empty()),
            ("surfaces", self.surfaces.is_empty()),
            ("models", self.models.is_empty()),
        ] {
            if empty {
                return Err(fault(
                    field,
                    inventory,
                    "inventory section must not be empty",
                ));
            }
        }

        let mut sources = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for source in &self.sources {
            let id = source.source_id.as_str();
            nonblank("sources.source_id", id, id)?;
            if !sources.insert(id) {
                return Err(fault("sources.source_id", id, "repeated source identity"));
            }
            if !source_path(&source.path) {
                return Err(fault(
                    "sources.path",
                    id,
                    "source path must be a safe portable repository-relative path",
                ));
            }
            if !paths.insert(source.path.as_str()) {
                return Err(fault("sources.path", id, "repeated source path"));
            }
            if !source_digest(&source.sha256) {
                return Err(fault(
                    "sources.sha256",
                    id,
                    "source digest must be 64 lowercase hexadecimal characters",
                ));
            }
        }

        let mut surfaces = BTreeMap::new();
        for surface in &self.surfaces {
            surface_fields(surface, &sources)?;
            if surfaces
                .insert(surface.surface_id.as_str(), surface)
                .is_some()
            {
                return Err(fault(
                    "surfaces.surface_id",
                    &surface.surface_id,
                    "repeated surface identity",
                ));
            }
        }
        surface_relations(&surfaces)?;

        let mut models = BTreeSet::new();
        for model in &self.models {
            let id = model.model_id.as_str();
            for (field, value) in [
                ("models.model_id", id),
                ("models.name", model.name.as_str()),
                ("models.owner", &model.owner),
                ("models.carrier", &model.carrier),
                ("models.schema_ref", &model.schema_ref),
                ("models.target", &model.target),
                ("models.authority", &model.authority),
                ("models.completeness", &model.completeness),
                ("models.retention", &model.retention),
            ] {
                nonblank(field, id, value)?;
            }
            if !models.insert(id) {
                return Err(fault("models.model_id", id, "repeated model identity"));
            }
            unique_strings(&model.effects, "models.effects", id)?;
            unique_strings(&model.conformance_cases, "models.conformance_cases", id)?;
            source_refs(&model.source_ids, &sources, "models.source_ids", id)?;
        }

        let mut families = BTreeSet::new();
        for family in &self.planned_families {
            let id = family.family_id.as_str();
            for (field, value) in [
                ("planned_families.family_id", id),
                ("planned_families.name", family.name.as_str()),
                ("planned_families.owner", &family.owner),
                ("planned_families.reason", &family.reason),
            ] {
                nonblank(field, id, value)?;
            }
            if !families.insert(id) {
                return Err(fault(
                    "planned_families.family_id",
                    id,
                    "repeated planned family identity",
                ));
            }
            source_refs(
                &family.source_ids,
                &sources,
                "planned_families.source_ids",
                id,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "capability_coverage/tests.rs"]
mod tests;
