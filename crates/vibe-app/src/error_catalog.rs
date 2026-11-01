//! Errors of the user layer over the launcher catalog
//! (`PROP-059#launcher-catalog`): a key that names no launcher of the home,
//! the two refusals about a collection's name, and a stop of something that
//! is not running (`PROP-059#launcher-runtime`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use specmark::spec;
use thiserror::Error;

/// The user layer over the launcher catalog: collections, pins and
/// per-launcher overrides (`PROP-059#launcher-catalog`).
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub enum CatalogError {
    #[error(
        "launcher_missing: `{key}` names no launcher of {home} (violates spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog; \
         fix: take a `key` from `vibe app launchers --all --json`, spelled <group>/<name>@<version>#<launcher id>)"
    )]
    LauncherMissing {
        key: String,
        /// `the user home` or `the project home <path>`.
        home: String,
    },
    #[error(
        "collection_missing: no collection `{name}` exists in {home} (violates spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog; \
         fix: list them with `vibe app collection list`, or create it with `vibe app collection create`)"
    )]
    CollectionMissing { name: String, home: String },
    #[error(
        "collection_exists: a collection `{name}` already exists (violates spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog; \
         fix: add to it with `vibe app collection add`, or choose another name)"
    )]
    CollectionExists { name: String },
    #[error(
        "not_running: no launcher started through vibe in {home} is running as `{target}` (violates spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime; \
         fix: take a `key` or `pid` from `vibe app running --json`)"
    )]
    NotRunning { target: String, home: String },
}
