//! Address conversion: `spec://` fields, authorities, and document identity.

use crate::{Authority, SpecAddress};

use super::super::ir::{DocumentAddress, SourceFormatId, SourceIr};
use super::bounded::{display, preview};
use super::{G_ADDRESS_REPARSE, G_SCALAR_IDS, IrWireError, gate, require_scalar, wire};

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

/// The source-level payload: typed address, open frontend identity, raw text.
pub(super) fn decode_source_doc(value: &wire::SourceDoc) -> Result<SourceIr, IrWireError> {
    let address = decode_document_address(&value.address)?;
    require_scalar("source format", value.format.as_str())?;
    let format = SourceFormatId::new(value.format.clone())
        .map_err(|_| gate(G_SCALAR_IDS, "source format must not be blank"))?;
    Ok(SourceIr::new(address, format, value.text.clone()))
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
        text: value.text().to_string(),
    }
}
