//! The journal's projector — fold a stream of registry facts into the
//! catalog they describe (PROP-044 §3: the journal is the truth, the
//! catalog a projection that can be torn down and rebuilt at any
//! moment).
//!
//! [`project`] is the whole module: pure, order-driven, total where the
//! journal is honest and loud where it is not. Everything the resulting
//! [`Index`] carries comes from the events — no clock, no IO, no
//! reading of an existing catalog — which is what makes "tear the
//! catalog down and rebuild it" a real verification rather than a
//! comparison of two guesses (PROP-044 §4.4). Reading the records off
//! disk is [`super::store::replay`]'s job; this fold never leaves
//! memory.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::collections::BTreeMap;

use semver::Version;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::index::memory::PkgKey;
use crate::journal::record::{Event, JournalRecord};
use crate::types::{NamingConvention, PackageEntry, VersionEntry};

/// Fold a journal into the catalog it describes.
///
/// Pure: no clock, no IO, no reading of the catalog. Everything the
/// result carries comes from the events — which is what makes
/// "tear the catalog down and rebuild it" a real verification rather
/// than a comparison of two guesses (PROP-044 §4.4).
///
/// The fold semantics, variant by variant:
///
/// - `Initialised` sets the registry identity; the LAST record wins —
///   a re-initialisation is a real fact, and last-one-wins is this
///   fold's call, not any writer's (see `cli/init.rs`).
/// - `Published` inserts or replaces the version, with exactly the
///   semantics of [`Index::upsert`].
/// - `Removed` with `Some(v)` drops that version (the package row
///   stays, per [`Index::remove_version`]); with `None` it drops the
///   whole package.
/// - `EntrySetReplaced` clears ONLY the entry set (`by_pkgref`) —
///   tombstones and every other journal-carried fact survive the
///   watershed, because a scan cannot re-derive them and dropping them
///   would make state unrecoverable (PROP-044 law 2).
/// - `Yanked` / `Frozen` set their flag on the named version iff it
///   stands in the projection at that point of the fold (see
///   [`mark_version`]).
///
/// `generated_at` is the `at` of the LAST applied record: the value is
/// derivable from the events themselves, so the function stays pure —
/// a clock argument would make it depend on the edge, and the plan's
/// appendix A.3 requires the fold to run without one. The meaning is
/// honest: the catalog reflects the journal up to that moment.
///
/// `schema_version` is the writer's own constant, stamped by
/// [`Index::new`]: the F2.2 rule forbids overwriting a version the
/// writer READ, and a projection reads nothing — it births a catalog
/// from scratch, so its label is truthful. Compatibility lives a floor
/// above, in the journal's epoch and each record's `must_understand`.
pub fn project(events: impl IntoIterator<Item = JournalRecord>) -> Result<Index> {
    // The fold state is a real `Index`, born placeholder-empty on the
    // first record and filled by the events themselves. Entry-set
    // semantics therefore route through the real Index methods rather
    // than a re-implementation of them.
    let mut folded: Option<Index> = None;
    let mut initialised = false;
    for record in events {
        let at = record.at;
        // The catalog reflects the journal up to this record. The birth
        // stamp below is the same `at`, so the running assignment is
        // what survives to the end — the last applied record's `at`.
        let idx = folded.get_or_insert_with(|| {
            // Placeholder identity, never observable: an `Initialised`
            // record overwrites it, and a journal without one errors
            // below before this index can escape. `Fqdn` matches the
            // CLI's own default for a not-yet-decided identity.
            Index::new("", "", NamingConvention::Fqdn, at)
        });
        idx.generated_at = at;
        match record.event {
            Event::Initialised {
                registry,
                registry_url,
                naming,
            } => {
                idx.registry = registry;
                idx.registry_url = registry_url;
                idx.naming = naming;
                initialised = true;
            }
            Event::Published { entry } => {
                idx.upsert(*entry);
            }
            Event::Removed {
                group,
                name,
                version,
            } => match version {
                Some(version) => {
                    idx.remove_version(&group, &name, &version);
                }
                None => {
                    idx.remove_package(&group, &name);
                }
            },
            Event::EntrySetReplaced { .. } => {
                // The watershed: the entry set is the scan product and
                // goes; every other carrier stays (see fn doc).
                idx.by_pkgref.clear();
            }
            Event::Yanked {
                group,
                name,
                version,
                ..
            } => {
                mark_version(&mut idx.by_pkgref, &group, &name, &version, |e| {
                    e.yanked = true
                });
            }
            Event::Frozen {
                group,
                name,
                version,
                ..
            } => {
                mark_version(&mut idx.by_pkgref, &group, &name, &version, |e| {
                    e.frozen = true
                });
            }
            Event::Renamed { .. } => {
                return Err(unprojectable("Renamed", "package rename"));
            }
            Event::Notice { .. } => {
                return Err(unprojectable("Notice", "per-package notice"));
            }
            Event::ChannelSet { .. } => {
                return Err(unprojectable("ChannelSet", "channels"));
            }
            Event::ChannelUnset { .. } => {
                return Err(unprojectable("ChannelUnset", "channels"));
            }
            Event::ForceReplaced { .. } => {
                return Err(unprojectable("ForceReplaced", "forced content replacement"));
            }
        }
    }

    let Some(index) = folded else {
        return Err(Error::Unprojectable(
            "the journal is empty — a catalog folded from it would carry no \
             registry identity, and inventing one would assert a state the \
             journal does not describe"
                .into(),
        ));
    };
    if !initialised {
        return Err(Error::Unprojectable(
            "the journal carries no `Initialised` record — a catalog folded \
             from it would carry no registry identity, and inventing one \
             would assert a state the journal does not describe"
                .into(),
        ));
    }
    Ok(index)
}

/// Set a per-version journal fact (`yanked`, `frozen`) on the named
/// version, iff it stands in the projection at this point of the fold.
///
/// Journal ORDER decides whether the target is there, and a missing
/// target is not an error: an earlier `Removed` may already have
/// retired the version — the fact arrives late and changes nothing
/// observable — or it was never published, and there is nothing to
/// mark. Either way the catalog asserts no state the journal fails to
/// describe, while erroring would make such journals unfoldable and
/// break rebuild-from-truth at exactly the moment it is needed.
fn mark_version(
    by_pkgref: &mut BTreeMap<PkgKey, PackageEntry>,
    group: &Group,
    name: &str,
    version: &Version,
    mark: impl FnOnce(&mut VersionEntry),
) {
    let Some(pkg) = by_pkgref.get_mut(&(group.clone(), name.to_string())) else {
        return;
    };
    if let Some(entry) = pkg.versions.iter_mut().find(|v| &v.version == version) {
        mark(entry);
    }
}

/// The journal names a fact whose carrier this build has not built
/// (this fold covers exactly six variants). Refusing is deliberate:
/// the journal is the truth (PROP-044 §3), so silently skipping one of
/// its facts would project a catalog the journal does not describe.
/// The refusal is a gate — the day a writer first records such an
/// event, projection stops with a nameable cause instead of quietly
/// diverging. Their handling arrives with the slices those carriers
/// belong to.
fn unprojectable(variant: &str, carrier: &str) -> Error {
    Error::Unprojectable(format!(
        "the journal holds a `{variant}` record, but its carrier ({carrier}) \
         is not built in this vibe-index — skipping the record would project \
         a catalog the journal does not describe"
    ))
}
