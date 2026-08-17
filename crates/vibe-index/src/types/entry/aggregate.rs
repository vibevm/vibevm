//! Aggregate records built over [`VersionEntry`](super::VersionEntry)
//! — re-exports of the generated wire types of the `by_name` schema:
//! [`PackageEntry`] gathers every indexed version of one
//! `(group, name)` identity (PROP-008 §2.2); [`NameEntry`] gathers
//! every `PackageEntry` that shares one bare `name` — the
//! `by-name/<name>.json` candidate set that makes short-name
//! resolution one round-trip per registry. The root type's name is
//! declared in the schema (`x-rust-type`), not derived from the file
//! name.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry");

pub use vibe_wire::generated::index::e1::by_name::{NameEntry, PackageEntry, Tombstone};
