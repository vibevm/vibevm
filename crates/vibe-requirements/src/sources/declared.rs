//! Descriptor-owned requirements source from a lock-selected Mixed slot.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use vibe_core::manifest::{Manifest, SourceSelector, SpecFormat};
use vibe_facts::{CapturedDeclaredDocument, SourceFileWitness, observe_declared_source};
use vibe_safefs::Project;
use vibe_wire::generated::requirements_report::SourceResultState;
use vibe_workspace::vibedeps;

use super::{
    MANIFEST_CAP, REASON_DECLARED_INCOMPLETE, REASON_DECLARED_LIMIT, REASON_DECLARED_TRANSFORMED,
    REASON_DECLARED_UNREADABLE, REASON_DECLARED_UNVERIFIED, RECORD_CAP, SOURCE_CAP, SourceCoord,
    SourceOutcome, source_result,
};
use crate::digest::source_result_digest;

const MAX_DECLARED_FILES: usize = 512;
const MAX_RETAINED_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourcePreflight {
    Limit,
    Unreadable,
}

/// Size metadata is a resource ceiling, never source authority. The pinned
/// Project read still rechecks each object and enforces its own per-file cap.
fn preflight_resources(
    root: &Path,
    paths: &BTreeSet<String>,
    max_files: usize,
    max_bytes: u64,
) -> Result<BTreeSet<String>, ResourcePreflight> {
    if paths.len() > max_files {
        return Err(ResourcePreflight::Limit);
    }
    let mut total = 0_u64;
    let mut present = BTreeSet::new();
    for path in paths {
        let metadata = match fs::symlink_metadata(root.join(path)) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(ResourcePreflight::Unreadable),
        };
        if !metadata.file_type().is_file() {
            return Err(ResourcePreflight::Unreadable);
        }
        present.insert(path.clone());
        if metadata.len() > SOURCE_CAP as u64 {
            return Err(ResourcePreflight::Limit);
        }
        total = total
            .checked_add(metadata.len())
            .ok_or(ResourcePreflight::Limit)?;
        if total > max_bytes {
            return Err(ResourcePreflight::Limit);
        }
    }
    Ok(present)
}

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
                // In-place has no immutable materialized record. Its CURRENT
                // pinned manifest is live authority under the owner's weaker
                // cooperative contract: removing descriptors can select the
                // legacy route. This is not lock/content-CAS proof.
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

    let mut paths = BTreeSet::new();
    for descriptor in &package.sources {
        let SourceSelector::File(path) = descriptor.selector() else {
            unreachable!()
        };
        paths.insert(path.clone());
    }
    let present = match preflight_resources(root, &paths, MAX_DECLARED_FILES, MAX_RETAINED_BYTES) {
        Ok(present) => present,
        Err(ResourcePreflight::Limit) => {
            return Some(declared_unavailable(coord, REASON_DECLARED_LIMIT));
        }
        Err(ResourcePreflight::Unreadable) => {
            return Some(declared_unavailable(coord, REASON_DECLARED_UNREADABLE));
        }
    };
    let mut captured = Vec::<(String, Option<Vec<u8>>)>::with_capacity(paths.len());
    let mut retained = 0_u64;
    for path in paths {
        let remaining = MAX_RETAINED_BYTES.saturating_sub(retained);
        let cap = SOURCE_CAP.min(usize::try_from(remaining).unwrap_or(usize::MAX));
        let bytes = match pinned.read_file_snapshot_bounded(&path, cap) {
            Ok(Some(snapshot)) => {
                retained = retained.checked_add(snapshot.size).unwrap_or(u64::MAX);
                if retained > MAX_RETAINED_BYTES {
                    return Some(declared_unavailable(coord, REASON_DECLARED_LIMIT));
                }
                if !record
                    .files
                    .iter()
                    .any(|row| row.path == path && row.sha256 == snapshot.sha256)
                {
                    return Some(unavailable());
                }
                Some(snapshot.bytes)
            }
            Ok(None) if present.contains(&path) => {
                return Some(declared_unavailable(coord, REASON_DECLARED_UNREADABLE));
            }
            Ok(None) => None,
            Err(_) => return Some(declared_unavailable(coord, REASON_DECLARED_UNREADABLE)),
        };
        captured.push((path, bytes));
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
    if !observed.complete {
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

#[cfg(test)]
mod tests {
    use super::{ResourcePreflight, preflight_resources};
    use std::collections::BTreeSet;
    use std::fs;

    #[test]
    fn file_count_and_aggregate_caps_refuse_before_retaining_source_bytes() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.md"), b"abc").unwrap();
        fs::write(root.path().join("b.md"), b"def").unwrap();
        let paths = BTreeSet::from(["a.md".to_owned(), "b.md".to_owned()]);
        assert_eq!(
            preflight_resources(root.path(), &paths, 1, 6),
            Err(ResourcePreflight::Limit)
        );
        assert_eq!(
            preflight_resources(root.path(), &paths, 2, 5),
            Err(ResourcePreflight::Limit)
        );
        assert_eq!(preflight_resources(root.path(), &paths, 2, 6), Ok(paths));
    }
}
