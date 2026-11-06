//! Bind a named export to the actual descriptor-selected captured snapshot.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-EXPORT-RESOLUTION");
use super::{DeclaredDocumentObservation, DeclaredSourceDisposition};
use crate::SourceFileWitness;
use vibe_core::manifest::{PackageMeta, SelectedSource, SourceInterpretation};
use vibe_specdoc::commonmark::{HeadingSelectionError, SectionSelection};

/// Named identity and host witness alongside a document-borrowed section.
/// This carries neither normative fact identity nor filesystem authority.
#[derive(Debug)]
pub struct DeclaredExportResolution<'a> {
    pub package_coordinate: String,
    pub export_name: String,
    pub resource_identity: String,
    pub witness: SourceFileWitness,
    pub selection: SectionSelection<'a>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredExportError {
    UndeclaredExport,
    DuplicateExportName,
    UnselectedSource,
    StructureUnavailable,
    WrongResource,
    ChangedWitness,
    Selector(HeadingSelectionError),
}

/// Resolve a manifest name against the same parsed captured document; no read,
/// reparsing, suffix selection, new hash policy or normative fact is involved.
/// ```
/// use vibe_core::manifest::PackageMeta;
/// use vibe_facts::{observe_declared_document, resolve_declared_export};
/// let package: PackageMeta = toml::from_str("name='guide'\ngroup='org.demo'\nkind='flow'\nversion='1.0.0'\n[[sources]]\nfile='guide.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='explicit'\n[[source_exports]]\nname='intro'\nfile='guide.md'\nheadings=['Guide']\n").unwrap();
/// let captured = observe_declared_document(&package, "guide.md", Some(b"# Guide\nLiteral @fact:WORD.\n")).unwrap();
/// let selected = resolve_declared_export(&package, "intro", &captured).unwrap();
/// assert_eq!(selected.export_name, "intro");
/// assert!(selected.selection.raw().contains("@fact:WORD"));
/// ```
pub fn resolve_declared_export<'a>(
    package: &PackageMeta,
    name: &str,
    observation: &'a DeclaredDocumentObservation<'_>,
) -> Result<DeclaredExportResolution<'a>, DeclaredExportError> {
    let mut matching = package
        .source_exports
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(|row| row.name() == name);
    let export = matching
        .next()
        .ok_or(DeclaredExportError::UndeclaredExport)?;
    if matching.next().is_some() {
        return Err(DeclaredExportError::DuplicateExportName);
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
    let DeclaredDocumentObservation::OrdinaryMarkdown {
        provenance,
        bytes,
        structure: Ok(document),
    } = observation
    else {
        return Err(DeclaredExportError::StructureUnavailable);
    };
    if provenance.identity != identity
        || provenance.reader != Some("vibe-specdoc:commonmark")
        || !matches!(&provenance.disposition, DeclaredSourceDisposition::Selected { declaration_index: index, descriptor: row } if *index == declaration_index && row == descriptor)
    {
        return Err(DeclaredExportError::WrongResource);
    }
    let witness = provenance
        .witness
        .as_ref()
        .ok_or(DeclaredExportError::ChangedWitness)?;
    if witness.path != export.file()
        || document.source().as_bytes() != *bytes
        || witness != &SourceFileWitness::of(export.file(), bytes)
    {
        return Err(DeclaredExportError::ChangedWitness);
    }
    let selection = document
        .resolve_headings(export.headings())
        .map_err(DeclaredExportError::Selector)?;
    Ok(DeclaredExportResolution {
        package_coordinate: format!("{}/{}", package.group, package.name),
        export_name: export.name().to_owned(),
        resource_identity: identity,
        witness: witness.clone(),
        selection,
    })
}

#[cfg(test)]
mod tests;
