//! Check — every installed slot holds the payload `vibe.lock` pins.
//!
//! Two records name the bytes of an installed package: the lock row's
//! `content_hash` and the slot record's `source_hash`, which the
//! materialiser writes beside the copied files. Install writes both from
//! the same fetch, so they agree; a path that rewrites one without the
//! other leaves a project whose lock no longer describes what is installed,
//! and the next install verified against the lock meets a fingerprint that
//! does not match (B-196). Nothing else compares the two, so this cell
//! does. An `Error`: the lock is the reproducibility contract, and a lock
//! that is wrong about the tree beside it is broken, not merely stale.
//!
//! A slot without a record — an in-place clone, a slot from before records
//! existed, a slot that is missing altogether — has nothing to compare;
//! [`LockfileFiles`](crate::CheckId::LockfileFiles) owns a missing slot.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{FieldPath, Lockfile, SourceKind};
use vibe_workspace::vibedeps::{SLOT_RECORD_FILENAME, read_slot_record, slot_rel_path};

use crate::{Check, CheckId, CheckOptions, CheckReport, Finding, Severity};

/// The [`CheckId::LockSlotHash`] cell.
#[cell(seam = "Check", variant = "lock-slot-hash")]
pub struct LockSlotHashCheck;

impl Check for LockSlotHashCheck {
    fn id(&self) -> CheckId {
        CheckId::LockSlotHash
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let lockfile_path = project_root.join(Lockfile::FILENAME);
        if !lockfile_path.exists() {
            return;
        }
        let Ok(lockfile) = Lockfile::read(&lockfile_path) else {
            // Surfaced by the manifest-validity cell; don't double-report.
            return;
        };
        for (index, pkg) in lockfile.packages.iter().enumerate() {
            let slot_rel = slot_rel_path(&pkg.group, &pkg.name, &pkg.version);
            let slot = project_root.join(&slot_rel);
            if !slot.join(SLOT_RECORD_FILENAME).is_file() {
                continue;
            }
            // A record that does not read is a damaged slot, which the
            // installer's own verification refuses; this cell answers only
            // whether two readable records agree.
            let Ok(record) = read_slot_record(&slot) else {
                continue;
            };
            let slot_hash = record.source_hash.as_str();
            let locked = pkg.content_hash.as_str();
            if slot_hash == locked {
                continue;
            }
            // Bytes a registry, a git source or an override pins are an
            // integrity record: disagreement is an error. A source that lives
            // in a working tree (`local`, `path`, `embedded`) changes as people
            // edit it, and `vibe install` re-records it on purpose — the same
            // drift `local_source_freshness` already reports as a warning.
            let in_tree = matches!(
                pkg.source_kind,
                Some(SourceKind::Local | SourceKind::Path | SourceKind::Embedded)
            );
            report.push(Finding {
                check: CheckId::LockSlotHash,
                severity: if in_tree {
                    Severity::Warning
                } else {
                    Severity::Error
                },
                path: Some(PathBuf::from(&slot_rel)),
                line: None,
                column: None,
                // The lock row, addressed the way C-1 addresses a manifest
                // field, so a reader can find which row disagrees without
                // parsing the message.
                field: Some(
                    FieldPath::root()
                        .table("package")
                        .index(index)
                        .field("content_hash")
                        .to_string(),
                ),
                message: format!(
                    "package {group}/{name}@{version}: the installed slot holds source \
                     {slot_hash}, but vibe.lock pins {locked} — run \
                     `vibe update {group}/{name}` to re-resolve and record one payload",
                    group = pkg.group,
                    name = pkg.name,
                    version = pkg.version,
                ),
            });
        }
    }
}

#[cfg(test)]
#[path = "lock_slot_hash/tests.rs"]
mod tests;
