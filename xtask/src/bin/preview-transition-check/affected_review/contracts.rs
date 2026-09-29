//! Captured manifest authority and pure URI/node resolution, never ambient lookup.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#AFFECTED-SOURCE-REVIEW");

use super::{
    CheckFailure, CheckReport, Context, FindingState, capture, fault, git_entry, paths, pending,
};
use std::collections::{BTreeMap, BTreeSet};
use vibe_wire::generated::preview::e1::affected_source_review::{
    AffectedSourceReview, ContractSource,
};

struct Source {
    inventory: BTreeMap<String, Vec<(String, bool)>>,
    documents: BTreeMap<String, vibe_specdoc::doc::SpecDoc>,
    declared_documents: BTreeSet<String>,
    authority_verified: bool,
    profile_supported: bool,
    inventory_complete: bool,
}

fn relative<'a>(path: &'a str, root: &str) -> Option<&'a str> {
    if root.is_empty() {
        Some(path)
    } else {
        path.strip_prefix(root)?.strip_prefix('/')
    }
}

fn source(
    binding: &ContractSource,
    context: &Context<'_>,
    review_id: &str,
    report: &mut CheckReport,
) -> Result<Source, CheckFailure> {
    let mut authority_verified = false;
    let bytes = paths::required_blob(
        context,
        &binding.manifest_path,
        None,
        &binding.manifest_sha256,
        review_id,
        report,
    )?;
    if let Some(bytes) = bytes {
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            fault(
                "affected-contract-manifest-encoding",
                review_id,
                "Captured manifest is not UTF-8",
            )
        })?;
        let manifest = vibe_core::manifest::Manifest::parse_str(text).map_err(|_| {
            fault(
                "affected-contract-manifest",
                review_id,
                "Canonical manifest owner refused captured source",
            )
        })?;
        let authority = match (&manifest.project, &manifest.package) {
            (Some(project), None) => project
                .group
                .as_ref()
                .map(|group| (group.to_string(), project.name.clone())),
            (None, Some(package)) => Some((package.group.to_string(), package.name.to_string())),
            _ => None,
        };
        if authority != Some((binding.group.clone(), binding.name.clone())) {
            return Err(fault(
                "affected-contract-authority",
                review_id,
                "Captured manifest authority differs from the reviewed source binding",
            ));
        }
        report.checked_claims.push(format!(
            "affected-contract-manifest-authority:{review_id}:{}",
            binding.manifest_path
        ));
        authority_verified = true;
    }

    let profile = binding.source_profile == "urn:vibevm:spec:xml";
    if !profile {
        pending(
            report,
            "affected-contract-profile-unsupported",
            review_id,
            "Declared contract source profile has no supported owner in this atom",
            FindingState::Unsupported,
        );
    }
    let root = binding.spec_root_components.join("/");
    let mut args = vec![
        "ls-tree",
        "-r",
        "-z",
        "--full-tree",
        context.after.repository_commit_sha1.as_str(),
    ];
    if !root.is_empty() {
        args.extend(["--", root.as_str()]);
    }
    let raw = capture::git(context.root, &args)?;
    let mut inventory: BTreeMap<String, Vec<(String, bool)>> = BTreeMap::new();
    let mut inventory_complete = true;
    for raw in git_entry::rows(&raw) {
        let Some(entry) = paths::row(raw, review_id, report)? else {
            inventory_complete = false;
            continue;
        };
        let Some(path) = relative(entry.repository_path, &root) else {
            return Err(fault(
                "affected-contract-spec-root",
                review_id,
                "Observed document escaped the explicit reviewed specification root",
            ));
        };
        if !path.ends_with(".xml") && !path.ends_with(".md") {
            continue;
        }
        if !entry.regular() {
            pending(
                report,
                "affected-contract-entry-unsupported",
                review_id,
                "A relevant contract candidate is nonregular and is not followed",
                FindingState::Unsupported,
            );
        }
        inventory
            .entry(vibe_spec::canonical_doc_path(path))
            .or_default()
            .push((entry.repository_path.into(), entry.regular()));
    }
    let mut documents = BTreeMap::new();
    for document in &binding.documents {
        let Some(bytes) = paths::required_blob(
            context,
            &document.path,
            Some(&document.git_blob_sha1),
            &document.sha256,
            review_id,
            report,
        )?
        else {
            continue;
        };
        report.checked_claims.push(format!(
            "affected-contract-document-bytes:{review_id}:{}",
            document.path
        ));
        if !profile {
            continue;
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            fault(
                "affected-contract-xml-encoding",
                review_id,
                "Captured contract XML is not UTF-8",
            )
        })?;
        let doc = vibe_specdoc::from_xml(text).map_err(|error| {
            CheckFailure::new(
                "affected-contract-xml",
                review_id,
                format!(
                    "Canonical Spec XML reader refused captured source line {}",
                    error.line
                ),
            )
        })?;
        documents.insert(document.path.clone(), doc);
    }
    Ok(Source {
        inventory,
        documents,
        declared_documents: binding
            .documents
            .iter()
            .map(|doc| doc.path.clone())
            .collect(),
        authority_verified,
        profile_supported: profile,
        inventory_complete,
    })
}

pub(super) fn verify(
    review: &AffectedSourceReview,
    context: &Context<'_>,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let mut sources = BTreeMap::new();
    for binding in &review.contract_sources {
        let verified = source(binding, context, &review.review_id, report)?;
        sources.insert((binding.group.clone(), binding.name.clone()), verified);
    }
    let required: BTreeSet<_> = review
        .interfaces
        .iter()
        .flat_map(|interface| interface.contract_anchors.iter())
        .collect();
    for uri in required {
        let address = vibe_spec::SpecAddress::parse(uri).map_err(|_| {
            fault(
                "affected-contract-uri",
                &review.review_id,
                "Canonical SpecAddress owner refused contract URI",
            )
        })?;
        let vibe_spec::Authority::Package {
            group,
            name,
            version,
        } = &address.authority
        else {
            pending(
                report,
                "affected-contract-authority-unsupported",
                &review.review_id,
                "Contract authority has no bound package source",
                FindingState::Unsupported,
            );
            continue;
        };
        if version.is_some() || address.pinned_r.is_some() {
            pending(
                report,
                "affected-contract-pin-unsupported",
                &review.review_id,
                "Version/revision pin has no bound owner in this atom and was not silently dropped",
                FindingState::Unsupported,
            );
            continue;
        }
        let Some(source) = sources.get(&(group.clone(), name.clone())) else {
            pending(
                report,
                "affected-contract-authority-unsupported",
                &review.review_id,
                "Contract package authority is not selected by this finite captured review",
                FindingState::Unsupported,
            );
            continue;
        };
        let Some(candidates) = source.inventory.get(&address.doc_path) else {
            if source.authority_verified && source.profile_supported && source.inventory_complete {
                return Err(fault(
                    "affected-contract-document-missing",
                    &review.review_id,
                    "Bound contract source has no known captured canonical document",
                ));
            }
            if !source.inventory_complete {
                pending(
                    report,
                    "affected-contract-target-inventory-unsupported",
                    &review.review_id,
                    "Unrepresentable captured candidates prevent proving that the requested canonical document is absent",
                    FindingState::Unsupported,
                );
            }
            continue;
        };
        if candidates.len() != 1 {
            return Err(fault(
                "affected-contract-document-collision",
                &review.review_id,
                "More than one actual captured candidate names the canonical document",
            ));
        }
        let (path, regular) = &candidates[0];
        if !regular {
            continue;
        }
        if !source.declared_documents.contains(path) {
            return Err(fault(
                "affected-contract-document-unbound",
                &review.review_id,
                "Actual canonical document lacks a declared verified blob/digest binding",
            ));
        }
        if !source.authority_verified || !source.profile_supported {
            continue;
        }
        let Some(document) = source.documents.get(path) else {
            continue;
        };
        if address.anchor.is_empty()
            || vibe_doc::citations::anchor_text(document, &address.anchor).is_none()
        {
            return Err(fault(
                "affected-contract-anchor-missing",
                &review.review_id,
                "Bound captured document does not contain the required permanent anchor",
            ));
        }
        if source.inventory_complete {
            report.checked_claims.push(format!(
                "affected-contract-anchor:{}:{uri}",
                review.review_id
            ));
        } else {
            report.checked_claims.push(format!(
                "affected-contract-known-node:{}:{path}#{}",
                review.review_id,
                address.anchor.join(".")
            ));
        }
    }
    Ok(())
}
