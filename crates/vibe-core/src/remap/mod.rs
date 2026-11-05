//! Remap: replaceable artifact sources (`PROP-066`).
//!
//! A vibe installation has to work where the public hosts are unreachable,
//! filtered, or forbidden by policy. Every artifact vibe fetches already
//! carries a hash, so **where** the bytes are looked for is a replaceable
//! detail and **what** they are is not. A remap row changes only the
//! former; the digest check stays exactly where it was, so a remap can
//! make a fetch fail or succeed and can never change what is installed.
//!
//! The three pieces, lowest to highest:
//!
//! - [`row`] — one `[[remap]]` row and the per-file `[policy]` table.
//! - [`layers`] — where rows come from, including PROP-002's `[[mirror]]`
//!   and `[[override]]` tables read as rows.
//! - [`locator`] — the one place a declared source becomes an ordered list
//!   of places to try.
//!
//! And above them, the half a package brings:
//!
//! - [`package`] — the offer a vibepack carries: its `[[remap]]` rows, the
//!   `[remap]` description and priority above them, and the row identity a
//!   provider map compares on.
//! - [`consent`] — the per-home acceptance ledger, bound to a content
//!   hash, because the default is always a question.
//! - [`installed`] — the remap packages one home has installed.
//! - [`providers`] — the derived map of which accepted packages offer
//!   which row, which is what uninstall consults.
//!
//! Not here, deliberately: the reachability probe of `remap test` and the
//! transport of `export` / `import`, which open sockets and walk a
//! project, and the question itself, which needs a terminal. Those live
//! in the CLI; what lives here is everything that is a pure function of
//! files on disk.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#purpose");

pub mod active;
pub mod artifact;
pub mod consent;
pub mod installed;
pub mod layers;
pub mod locator;
pub mod package;
pub mod providers;
pub mod row;

pub use active::{active, install};
pub use artifact::{Artifact, ArtifactKind, Candidate, Explanation, Resolution, RowMatch, RowRef};
pub use consent::{
    CONSENT_PROTOCOL, ConsentError, ConsentLedger, ConsentRecord, HomeLock, OfferState,
    consent_path, lock, remap_dir,
};
pub use installed::{
    InstalledRemap, InstalledRemaps, installed_path, offer_from_manifest, offer_from_tree,
    package_dir, packages_dir,
};
pub use layers::{
    Layer, LoadedSource, default_raw_file_rows, env_remap_files, rows_from_file, rows_from_mirrors,
    rows_from_overrides, rows_from_settings,
};
pub use locator::{Locator, RemapRefusal};
pub use package::{KeptRow, RemapPackage, RemapSection, RowIdentity, strictest_policy};
pub use providers::{
    PROVIDERS_PROTOCOL, Probed, ProvidedRow, ProviderMap, Reachability, providers_path,
    reachability_path,
};
pub use row::{
    PolicySection, Provenance, RemapFileDecl, RemapKey, RemapParseError, RemapRow, RemapScope,
    RemapTarget, RowDecl, UnmatchedPolicy,
};

/// The error code a refused fetch carries (`PROP-066#row`): the fetch was
/// refused before any connection, by a `deny` row or by
/// `unmatched = "deny"`.
pub const REMAP_DENIED: &str = "remap_denied";

/// The `--remap <file>` flag's long name, and the environment variable
/// that carries the same list — named here so the composition root and
/// every diagnostic spell them the same way.
pub const REMAP_ENV: &str = "VIBE_REMAP";

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[cfg(test)]
#[path = "tests_packages.rs"]
mod tests_packages;

#[cfg(test)]
#[path = "tests_source_precedence.rs"]
mod tests_source_precedence;
