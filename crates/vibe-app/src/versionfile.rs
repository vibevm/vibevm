//! Per-project version files (PROP-075 `#VERSION-FILE-FORMATS`): the two
//! declared formats, the upward search from the working directory, the
//! resolution cache keyed by the modification time and size of every file
//! consulted, and the sidecar that records which files the home's installed
//! applications declare.
//!
//! Nothing here resolves a version against the index — that is
//! [`crate::activation`]; this module answers only "what does the nearest
//! declared version file of this directory say".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

#[path = "versionfile/cache.rs"]
mod cache;
#[path = "versionfile/declared.rs"]
mod declared;
#[path = "versionfile/formats.rs"]
mod formats;
#[path = "versionfile/search.rs"]
mod search;

pub use cache::{CACHE_PROTOCOL, clear as clear_cache, path as cache_path, resolve};
pub use declared::{Declared, VERSION_FILES_PROTOCOL, path as declared_path};
pub use formats::{
    RequestKind, ToolVersions, VersionRequest, is_channel_name, plain_line, tool_versions,
};
pub use search::{Consulted, Found, Search, search};

#[cfg(test)]
#[path = "versionfile/tests.rs"]
mod tests;
