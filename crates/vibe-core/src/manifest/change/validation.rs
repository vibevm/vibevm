//! Shared declaration validation, independent of surfaces or filesystem effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-DOCUMENT-ROLES");
use super::*;
use std::collections::BTreeSet;
fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|v| {
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
pub(super) fn validate(
    wire: Wire,
    context: &ChangeContext,
    directory: &Path,
    epoch: u32,
) -> Result<ChangeManifest, ChangeError> {
    if wire.schema != epoch {
        return Err(invalid(
            ChangeErrorKind::UnknownEpoch,
            "schema",
            "unsupported change epoch",
        ));
    }
    let id = ChangeId::parse(&wire.change.id)?;
    if wire.change.selected_node != context.selected_node {
        return Err(invalid(
            ChangeErrorKind::SelectedNode,
            "change.selected_node",
            "differs from trusted selected node",
        ));
    }
    if wire.change.title.trim().is_empty() || wire.change.slug.trim().is_empty() {
        return Err(invalid(
            ChangeErrorKind::Identity,
            "change.metadata",
            "title/slug must be nonblank",
        ));
    }
    declarant_path(directory).map_err(|_| {
        invalid(
            ChangeErrorKind::Path,
            "directory",
            "change home must be project-relative",
        )
    })?;
    if directory != context.directory(&id) {
        return Err(invalid(
            ChangeErrorKind::Identity,
            "directory",
            "does not match configured root and immutable ID",
        ));
    }
    if wire.origins.is_empty() {
        return Err(invalid(
            ChangeErrorKind::Origin,
            "origins",
            "at least one captured origin required",
        ));
    }
    let mut packages = BTreeSet::new();
    for origin in &wire.origins {
        let pin = crate::PackageRef::parse(&origin.package).map_err(|_| {
            invalid(
                ChangeErrorKind::Origin,
                "origins.package",
                "invalid package pin",
            )
        })?;
        if !pin.is_qualified() || pin.kind.is_none() || !pin.version.is_exact_pin() {
            return Err(invalid(
                ChangeErrorKind::Origin,
                "origins.package",
                "exact qualified kind/group/name pin required",
            ));
        }
        if !packages.insert(origin.package.clone())
            || !digest(&origin.manifest_sha256)
            || origin.documents.is_empty()
        {
            return Err(invalid(
                ChangeErrorKind::Origin,
                "origins.capture",
                "duplicate/absent/invalid origin capture",
            ));
        }
        let mut addresses = BTreeSet::new();
        for doc in &origin.documents {
            let raw = doc
                .address
                .strip_prefix("spec://")
                .and_then(|v| v.split_once('/'))
                .ok_or_else(|| {
                    invalid(
                        ChangeErrorKind::Origin,
                        "origins.documents.address",
                        "qualified literal spec scheme required",
                    )
                })?;
            let raw_document = raw.1.split('#').next().unwrap();
            if raw.0 != pin.group.as_ref().unwrap().as_str()
                || declarant_path(Path::new(raw_document)).is_err()
            {
                return Err(invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.address",
                    "literal citation cannot normalize authority/path",
                ));
            }
            let url = url::Url::parse(&doc.address).map_err(|_| {
                invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.address",
                    "invalid spec citation",
                )
            })?;
            let group = pin.group.as_ref().unwrap().to_string();
            if url.scheme() != "spec"
                || url.host_str() != Some(group.as_str())
                || url.query().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
                || doc.address.contains('%')
                || doc.address.chars().any(char::is_whitespace)
            {
                return Err(invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.address",
                    "citation must name selected package without encoding/query",
                ));
            }
            let path = url.path().strip_prefix('/').unwrap_or(url.path());
            let (name, document) = path.split_once('/').ok_or_else(|| {
                invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.address",
                    "document citation required",
                )
            })?;
            if name != pin.name.as_str()
                || declarant_path(Path::new(document)).is_err()
                || document.ends_with(".md")
                || document.ends_with(".xml")
            {
                return Err(invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.address",
                    "citation coordinate/document differs from selected origin",
                ));
            }
            let anchor = url.fragment().ok_or_else(|| {
                invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.address",
                    "exact fact anchor required",
                )
            })?;
            if anchor.split('.').any(|segment| {
                segment.is_empty()
                    || !segment
                        .bytes()
                        .next()
                        .is_some_and(|b| b.is_ascii_alphabetic())
                    || !segment
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            }) {
                return Err(invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.address",
                    "invalid fact anchor path",
                ));
            }
            if !digest(&doc.sha256) || !addresses.insert(doc.address.clone()) {
                return Err(invalid(
                    ChangeErrorKind::Origin,
                    "origins.documents.capture",
                    "invalid/duplicate document capture",
                ));
            }
        }
    }
    let mut paths = BTreeSet::new();
    let mut intent = false;
    let mut acceptance = false;
    for doc in &wire.documents {
        declarant_path(Path::new(&doc.path)).map_err(|_| {
            invalid(
                ChangeErrorKind::Path,
                "documents.path",
                "escaping/nonportable role path",
            )
        })?;
        if !paths.insert(&doc.path) {
            return Err(invalid(
                ChangeErrorKind::DuplicatePath,
                "documents.path",
                "one physical declaration cannot serve multiple roles",
            ));
        }
        if (doc.role == ChangeDocumentRole::Closure && (doc.normative || doc.input))
            || (doc.role == ChangeDocumentRole::Work && doc.normative)
            || (doc.normative && !doc.input)
        {
            return Err(invalid(
                ChangeErrorKind::Role,
                "documents.role",
                "derived closure/work cannot be normative input",
            ));
        }
        intent |= doc.role == ChangeDocumentRole::Intent && doc.input && doc.normative;
        acceptance |= doc.role == ChangeDocumentRole::Acceptance && doc.input && doc.normative;
    }
    if !intent || !acceptance {
        return Err(invalid(
            ChangeErrorKind::MissingRole,
            "documents",
            "normative intent and acceptance inputs required",
        ));
    }
    Ok(ChangeManifest {
        id,
        wire,
        directory: directory.to_path_buf(),
        coordinate: context.coordinate.clone(),
    })
}
