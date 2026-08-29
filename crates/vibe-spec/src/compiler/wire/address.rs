//! Address conversion: `spec://` fields, authorities, and document identity.

use vibe_core::{Group, PackageName};

use crate::{Authority, SpecAddress};

use super::super::ir::{
    DocumentAddress, DocumentProvider, DocumentSubject, SourceFormatId, SourceIr,
};
use super::bounded::{display, preview};
use super::{
    G_ADDRESS_REPARSE, G_SCALAR_IDS, IrWireError, construction, gate, require_declared_path,
    require_scalar, wire,
};

/// Decode one `spec://` address, re-parsing its raw spelling against the
/// fields carried beside it (the address-reparse gate: raw drift is red).
pub(super) fn decode_spec_address(value: &wire::SpecAddress) -> Result<SpecAddress, IrWireError> {
    let authority = decode_authority(&value.authority)?;
    // A carried `raw` is attacker-sized; neither refusal below ever echoes it.
    let reparsed = SpecAddress::parse(&value.raw).map_err(|source| {
        gate(
            G_ADDRESS_REPARSE,
            format!(
                "raw address ({}) does not parse: {}",
                preview(&value.raw),
                display(source)
            ),
        )
    })?;
    let matches = reparsed.authority == authority
        && reparsed.doc_path == value.doc_path
        && reparsed.anchor == value.anchor
        && reparsed.pinned_r == value.pinned_r;
    if !matches {
        return Err(gate(
            G_ADDRESS_REPARSE,
            format!(
                "raw address ({}) re-parses to fields that differ from the carried spelling",
                preview(&value.raw)
            ),
        ));
    }
    Ok(SpecAddress {
        raw: value.raw.clone(),
        authority,
        doc_path: value.doc_path.clone(),
        anchor: value.anchor.clone(),
        pinned_r: value.pinned_r,
    })
}

/// The carried fields as a domain address, WITHOUT the raw-reparse gate.
/// The origin phase (gate 4) judges the package relation from the fields the
/// carrier declares beside `raw`; raw equality is the address phase's own
/// law and cannot fire first (repair 2, finding 3).
pub(super) fn carried_spec_address(value: &wire::SpecAddress) -> SpecAddress {
    SpecAddress {
        raw: value.raw.clone(),
        authority: carried_authority(&value.authority),
        doc_path: value.doc_path.clone(),
        anchor: value.anchor.clone(),
        pinned_r: value.pinned_r,
    }
}

fn carried_authority(value: &wire::Authority) -> Authority {
    match value {
        wire::Authority::Host(host) => Authority::Host(host.name.clone()),
        wire::Authority::Package(package) => Authority::Package {
            group: package.group.clone(),
            name: package.name.clone(),
            version: package.version.clone(),
        },
    }
}

fn decode_authority(value: &wire::Authority) -> Result<Authority, IrWireError> {
    match value {
        wire::Authority::Host(host) => {
            require_scalar("address authority host", &host.name)?;
            Ok(Authority::Host(host.name.clone()))
        }
        wire::Authority::Package(package) => {
            require_scalar("address authority group", &package.group)?;
            require_scalar("address authority name", &package.name)?;
            if let Some(version) = &package.version {
                require_scalar("address authority version", version)?;
            }
            Ok(Authority::Package {
                group: package.group.clone(),
                name: package.name.clone(),
                version: package.version.clone(),
            })
        }
    }
}

/// A document's compiler identity. A static entry's provider/path identity is
/// a scalar pair, so the ordinary scalar gate owns it.
pub(super) fn decode_document_address(
    value: &wire::DocumentAddress,
) -> Result<DocumentAddress, IrWireError> {
    match value {
        wire::DocumentAddress::Spec(spec) => {
            Ok(DocumentAddress::Spec(decode_spec_address(&spec.address)?))
        }
        wire::DocumentAddress::StaticEntry(entry) => {
            require_scalar("static entry origin", &entry.origin)?;
            require_scalar("static entry path", &entry.path)?;
            Ok(DocumentAddress::StaticEntry {
                origin: entry.origin.clone(),
                path: entry.path.clone(),
            })
        }
    }
}

/// The source-level payload: typed address, open frontend identity, immutable
/// document subject, raw text.
pub(super) fn decode_source_doc(value: &wire::SourceDoc) -> Result<SourceIr, IrWireError> {
    let address = decode_document_address(&value.address)?;
    require_scalar("source format", value.format.as_str())?;
    let format = SourceFormatId::new(value.format.clone())
        .map_err(|_| gate(G_SCALAR_IDS, "source format must not be blank"))?;
    let subject = decode_document_subject(&value.subject)?;
    Ok(SourceIr::new(address, format, subject, value.text.clone()))
}

/// One document's selector subject.
///
/// The member is REQUIRED on the wire, so a carrier that omits it is already
/// red at the strict reader: a decoder that defaulted it would silently decide
/// which transforms the document is in scope for. The path obeys the full
/// `paths` contract here, separator and all, because that is what a selector
/// dimension will be matched against.
pub(super) fn decode_document_subject(
    value: &wire::DocumentSubject,
) -> Result<DocumentSubject, IrWireError> {
    require_declared_path("subject declared path", &value.declared_path)?;
    let provider = decode_document_provider(&value.provider)?;
    Ok(DocumentSubject::declared(provider, &value.declared_path))
}

/// The typed provider a `packages` selector dimension would be matched
/// against.
///
/// The carrier is total: `unclaimed` and `undetermined` are modelled arms, not
/// a missing key, and they stay APART across the wire — the whole point of the
/// two spellings is that a plugin can tell "nothing declared this" from "the
/// declaring owner was not resolved". Every coordinate arm rebuilds through the
/// one validating constructor each component already has, so a carrier claiming
/// an ill-formed coordinate is refused by the domain law, never quietly stored
/// as a display string.
fn decode_document_provider(
    value: &wire::DocumentProvider,
) -> Result<DocumentProvider, IrWireError> {
    let provider = match value {
        wire::DocumentProvider::Unclaimed(_) => DocumentProvider::Unclaimed,
        wire::DocumentProvider::Undetermined(_) => DocumentProvider::Undetermined,
        wire::DocumentProvider::Dependency(arm) => DocumentProvider::Dependency {
            group: coordinate_group(&arm.group)?,
            name: coordinate_name(&arm.name)?,
        },
        wire::DocumentProvider::HostUngrouped(arm) => {
            require_scalar("subject provider host name", &arm.name)?;
            DocumentProvider::HostUngrouped {
                name: arm.name.clone(),
            }
        }
        wire::DocumentProvider::HostCoordinate(arm) => DocumentProvider::HostCoordinate {
            group: coordinate_group(&arm.group)?,
            name: coordinate_name(&arm.name)?,
        },
        wire::DocumentProvider::HostVirtualWorkspace(_) => DocumentProvider::HostVirtualWorkspace,
    };
    Ok(provider)
}

fn coordinate_group(value: &str) -> Result<Group, IrWireError> {
    require_scalar("subject provider group", value)?;
    Group::parse(value)
        .map_err(|source| construction(format!("subject provider group: {}", display(source))))
}

fn coordinate_name(value: &str) -> Result<PackageName, IrWireError> {
    require_scalar("subject provider name", value)?;
    PackageName::parse(value)
        .map_err(|source| construction(format!("subject provider name: {}", display(source))))
}

pub(super) fn encode_spec_address(value: &SpecAddress) -> wire::SpecAddress {
    wire::SpecAddress {
        raw: value.raw.clone(),
        authority: encode_authority(&value.authority),
        doc_path: value.doc_path.clone(),
        anchor: value.anchor.clone(),
        pinned_r: value.pinned_r,
    }
}

fn encode_authority(value: &Authority) -> wire::Authority {
    match value {
        Authority::Host(name) => {
            wire::Authority::Host(Box::new(wire::AuthorityHost { name: name.clone() }))
        }
        Authority::Package {
            group,
            name,
            version,
        } => wire::Authority::Package(Box::new(wire::AuthorityPackage {
            group: group.clone(),
            name: name.clone(),
            version: version.clone(),
        })),
    }
}

pub(super) fn encode_document_address(value: &DocumentAddress) -> wire::DocumentAddress {
    match value {
        DocumentAddress::Spec(address) => {
            wire::DocumentAddress::Spec(Box::new(wire::DocumentAddressSpec {
                address: encode_spec_address(address),
            }))
        }
        DocumentAddress::StaticEntry { origin, path } => {
            wire::DocumentAddress::StaticEntry(Box::new(wire::DocumentAddressStaticEntry {
                origin: origin.clone(),
                path: path.clone(),
            }))
        }
    }
}

pub(super) fn encode_source_doc(value: &SourceIr) -> wire::SourceDoc {
    wire::SourceDoc {
        address: encode_document_address(value.address()),
        format: value.format().as_str().to_string(),
        subject: encode_document_subject(value.subject()),
        text: value.text().to_string(),
    }
}

pub(super) fn encode_document_subject(value: &DocumentSubject) -> wire::DocumentSubject {
    wire::DocumentSubject {
        declared_path: value.declared_path().to_string(),
        provider: encode_document_provider(value.provider()),
    }
}

fn encode_document_provider(value: &DocumentProvider) -> wire::DocumentProvider {
    match value {
        DocumentProvider::Unclaimed => {
            wire::DocumentProvider::Unclaimed(Box::new(wire::DocumentProviderUnclaimed {}))
        }
        DocumentProvider::Undetermined => {
            wire::DocumentProvider::Undetermined(Box::new(wire::DocumentProviderUndetermined {}))
        }
        DocumentProvider::Dependency { group, name } => {
            wire::DocumentProvider::Dependency(Box::new(wire::DocumentProviderDependency {
                group: group.to_string(),
                name: name.as_str().to_string(),
            }))
        }
        DocumentProvider::HostUngrouped { name } => {
            wire::DocumentProvider::HostUngrouped(Box::new(wire::DocumentProviderHostUngrouped {
                name: name.clone(),
            }))
        }
        DocumentProvider::HostCoordinate { group, name } => {
            wire::DocumentProvider::HostCoordinate(Box::new(wire::DocumentProviderHostCoordinate {
                group: group.to_string(),
                name: name.as_str().to_string(),
            }))
        }
        DocumentProvider::HostVirtualWorkspace => wire::DocumentProvider::HostVirtualWorkspace(
            Box::new(wire::DocumentProviderHostVirtualWorkspace {}),
        ),
    }
}
