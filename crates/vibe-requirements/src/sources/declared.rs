//! Descriptor-owned requirements source from a lock-selected Mixed slot.

use std::collections::BTreeSet;
use std::path::Path;

use vibe_core::manifest::{Manifest, SourceSelector, SpecFormat};
use vibe_facts::{CapturedDeclaredDocument, SourceFileWitness, observe_declared_source};
use vibe_safefs::Project;
use vibe_wire::generated::requirements_report::SourceResultState;
use vibe_workspace::vibedeps;

use super::{
    MANIFEST_CAP, REASON_DECLARED_INCOMPLETE, REASON_DECLARED_TRANSFORMED,
    REASON_DECLARED_UNVERIFIED, RECORD_CAP, SOURCE_CAP, SourceCoord, SourceOutcome, source_result,
};
use crate::digest::source_result_digest;

fn declared_unavailable(coord: &SourceCoord, reason: &'static str) -> SourceOutcome {
    SourceOutcome {
        result: source_result(
            &coord.kind,
            &coord.package,
            SourceResultState::Unavailable,
            None,
            Some(reason.to_owned()),
            None,
        ),
        facts: Vec::new(),
    }
}

fn declared_invalid(coord: &SourceCoord, documents: &[SourceFileWitness]) -> SourceOutcome {
    SourceOutcome {
        result: source_result(
            &coord.kind,
            &coord.package,
            SourceResultState::Invalid,
            Some(source_result_digest(&coord.kind, &coord.package, documents)),
            Some(REASON_DECLARED_INCOMPLETE.to_owned()),
            None,
        ),
        facts: Vec::new(),
    }
}

/// `None` means a pinned, validated no-descriptor legacy package. Missing,
/// unsafe or malformed manifests never trigger the suffix route.
pub(super) fn observe_selected_declared(coord: &SourceCoord, root: &Path) -> Option<SourceOutcome> {
    let unavailable = || declared_unavailable(coord, REASON_DECLARED_UNVERIFIED);
    let pinned = match Project::open(root) {
        Ok(pinned) => pinned,
        Err(_) => return Some(unavailable()),
    };
    let manifest = match pinned.read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP) {
        Ok(Some(snapshot)) => snapshot,
        _ => return Some(unavailable()),
    };
    let manifest_text = match std::str::from_utf8(&manifest.bytes) {
        Ok(text) => text,
        Err(_) => return Some(unavailable()),
    };
    let value: toml::Value = match toml::from_str(manifest_text) {
        Ok(value) => value,
        Err(_) => return Some(unavailable()),
    };
    let authored = value
        .get("package")
        .and_then(toml::Value::as_table)
        .is_some_and(|table| table.contains_key("sources") || table.contains_key("source_exports"));
    let package = match Manifest::parse_str(manifest_text)
        .ok()
        .and_then(|m| m.package)
    {
        Some(package) => package,
        None => return Some(unavailable()),
    };
    let selected_version = match coord.selected_version.as_deref() {
        Some(version) => version,
        None => return Some(unavailable()),
    };
    let source_hash = match coord.content_hash.as_deref() {
        Some(hash) => hash,
        None => return Some(unavailable()),
    };
    if format!("{}/{}", package.group, package.name) != coord.package
        || package.version.to_string() != selected_version
    {
        return Some(unavailable());
    }
    let record_before =
        match pinned.read_file_snapshot_bounded(vibedeps::SLOT_RECORD_FILENAME, RECORD_CAP) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) if !authored && coord.in_place => {
                let after = pinned.read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP);
                return match after {
                    Ok(Some(after)) if after == manifest => None,
                    _ => Some(unavailable()),
                };
            }
            _ => return Some(unavailable()),
        };
    let record = match vibedeps::read_slot_record_bytes(
        &root.join(vibedeps::SLOT_RECORD_FILENAME),
        &record_before.bytes,
    ) {
        Ok(record) => record,
        Err(_) => return Some(unavailable()),
    };
    if record.source_hash.as_str() != source_hash {
        return Some(unavailable());
    }
    if !record
        .files
        .iter()
        .any(|row| row.path == Manifest::FILENAME && row.sha256 == manifest.sha256)
    {
        return Some(unavailable());
    }
    if !authored {
        let manifest_after = pinned.read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP);
        let record_after =
            pinned.read_file_snapshot_bounded(vibedeps::SLOT_RECORD_FILENAME, RECORD_CAP);
        return match (manifest_after, record_after) {
            (Ok(Some(manifest_after)), Ok(Some(record_after)))
                if manifest_after == manifest && record_after == record_before =>
            {
                None
            }
            _ => Some(unavailable()),
        };
    }
    if record.spec_format != SpecFormat::Mixed {
        return Some(declared_unavailable(coord, REASON_DECLARED_TRANSFORMED));
    }
    if package
        .sources
        .iter()
        .any(|row| matches!(row.selector(), SourceSelector::Directory(_)))
    {
        return Some(declared_unavailable(coord, REASON_DECLARED_INCOMPLETE));
    }

    let mut captured = Vec::<(String, Option<Vec<u8>>)>::new();
    let mut seen = BTreeSet::new();
    let mut unreadable = false;
    for descriptor in &package.sources {
        let SourceSelector::File(path) = descriptor.selector() else {
            unreachable!()
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        let bytes = match pinned.read_file_snapshot_bounded(path, SOURCE_CAP) {
            Ok(Some(snapshot)) => {
                if !record
                    .files
                    .iter()
                    .any(|row| row.path == *path && row.sha256 == snapshot.sha256)
                {
                    return Some(unavailable());
                }
                Some(snapshot.bytes)
            }
            Ok(None) => None,
            Err(_) => {
                unreadable = true;
                None
            }
        };
        captured.push((path.clone(), bytes));
    }
    let manifest_after = match pinned.read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP) {
        Ok(Some(snapshot)) => snapshot,
        _ => return Some(unavailable()),
    };
    let record_after =
        match pinned.read_file_snapshot_bounded(vibedeps::SLOT_RECORD_FILENAME, RECORD_CAP) {
            Ok(Some(snapshot)) => snapshot,
            _ => return Some(unavailable()),
        };
    if manifest != manifest_after || record_before != record_after {
        return Some(unavailable());
    }
    let mut witnesses = captured
        .iter()
        .filter_map(|(path, bytes)| {
            bytes
                .as_ref()
                .map(|bytes| SourceFileWitness::of(path, bytes))
        })
        .collect::<Vec<_>>();
    witnesses.sort_by(|left, right| left.path.cmp(&right.path));
    let inventory = captured
        .iter()
        .map(|(path, bytes)| CapturedDeclaredDocument {
            path,
            bytes: bytes.as_deref(),
        })
        .collect::<Vec<_>>();
    let observed = match observe_declared_source(&package, &inventory) {
        Ok(observed) => observed,
        Err(_) => return Some(declared_invalid(coord, &witnesses)),
    };
    if unreadable || !observed.complete {
        return Some(declared_invalid(coord, &witnesses));
    }
    Some(SourceOutcome {
        result: source_result(
            &coord.kind,
            &coord.package,
            SourceResultState::Available,
            Some(source_result_digest(
                &coord.kind,
                &coord.package,
                &witnesses,
            )),
            None,
            None,
        ),
        facts: observed.facts,
    })
}
