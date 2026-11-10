//! Per-project version files (PROP-075 `#VERSION-FILE-FORMATS`,
//! `#VERSION-FILE-KEYED`, `#VERSION-FILE-WRITE`): the declared formats,
//! the one keyed reader and the one keyed writer, the upward search from
//! the working directory, the
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
#[path = "versionfile/write.rs"]
mod write;

pub use cache::{CACHE_PROTOCOL, clear as clear_cache, path as cache_path, resolve};
pub use declared::{Declared, VERSION_FILES_PROTOCOL, path as declared_path};
pub use formats::{
    KeyedLine, KeyedSelection, RequestKind, VersionRequest, is_channel_name, keyed, keyed_lines,
    plain_line,
};
pub use search::{Consulted, Found, Search, search};
pub use write::{WriteRequest, Written, write};

#[cfg(test)]
#[path = "versionfile/tests.rs"]
mod tests;
