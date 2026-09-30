//! Join an authored export to the same descriptor-selected captured document.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-EXPORT-SELECTOR");

use std::ops::Range;

use vibe_core::manifest::{PackageMeta, SelectedSource, SourceExport, SourceInterpretation};
use vibe_specdoc::commonmark::{ExportResolutionError, NodeId};

use super::{DeclaredDocumentObservation, DeclaredReadProvenance, DeclaredSourceDisposition};
use crate::SourceFileWitness;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredExportResolution {
    /// Together with `export_name`, this is the durable authored identity.
    pub package_coordinate: String,
    pub export_name: String,
    pub resource_identity: String,
    /// Valid only within this document observation's parsed snapshot.
    pub node: NodeId,
    pub span: Range<usize>,
    pub witness: SourceFileWitness,
    pub provenance: DeclaredReadProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredExportError {
    UndeclaredExport,
    DuplicateExportName,
    UnselectedSource,
    IncompleteObservation,
    WrongResource,
    ChangedWitness,
    Selector(ExportResolutionError),
}

/// Resolve only a `Structure` observation from an exact selected source. No
/// filesystem access, reparse, fact address or ordinal fallback occurs here.
pub fn resolve_declared_export(
    package: &PackageMeta,
    export: &SourceExport,
    observation: &DeclaredDocumentObservation<'_>,
) -> Result<DeclaredExportResolution, DeclaredExportError> {
    let same_name = package
        .source_exports
        .iter()
        .filter(|row| row.name() == export.name())
        .collect::<Vec<_>>();
    if same_name.len() > 1 {
        return Err(DeclaredExportError::DuplicateExportName);
    }
    if same_name.first().is_none_or(|row| *row != export) {
        return Err(DeclaredExportError::UndeclaredExport);
    }
    let SelectedSource::Descriptor {
        identity,
        declaration_index,
        descriptor,
    } = package
        .select_source(export.file())
        .map_err(|_| DeclaredExportError::UnselectedSource)?
    else {
        return Err(DeclaredExportError::UnselectedSource);
    };
    if descriptor.interpretation() != SourceInterpretation::Commonmark
        || descriptor.media() != "text/markdown"
    {
        return Err(DeclaredExportError::UnselectedSource);
    }
    let DeclaredDocumentObservation::Structure {
        provenance,
        document,
    } = observation
    else {
        return Err(DeclaredExportError::IncompleteObservation);
    };
    if provenance.identity != identity || provenance.reader != Some("vibe-specdoc:commonmark") {
        return Err(DeclaredExportError::WrongResource);
    }
    if !matches!(&provenance.disposition,
        DeclaredSourceDisposition::Selected { declaration_index: observed_index, descriptor: observed_descriptor }
        if *observed_index == declaration_index && observed_descriptor == descriptor)
    {
        return Err(DeclaredExportError::WrongResource);
    }
    let witness = provenance
        .witness
        .as_ref()
        .ok_or(DeclaredExportError::IncompleteObservation)?;
    if witness.path != export.file() {
        return Err(DeclaredExportError::WrongResource);
    }
    let current = SourceFileWitness::of(export.file(), document.source().as_bytes());
    if *witness != current {
        return Err(DeclaredExportError::ChangedWitness);
    }
    let resolved = document
        .resolve_export(export.name(), export.headings())
        .map_err(DeclaredExportError::Selector)?;
    if witness.digest != resolved.source_sha256 {
        return Err(DeclaredExportError::ChangedWitness);
    }
    Ok(DeclaredExportResolution {
        package_coordinate: format!("{}/{}", package.group, package.name),
        export_name: export.name().to_owned(),
        resource_identity: identity,
        node: resolved.node,
        span: resolved.span,
        witness: witness.clone(),
        provenance: provenance.clone(),
    })
}

#[cfg(test)]
#[path = "export/tests.rs"]
mod tests;
