//! `shell.json` — what a shell says about itself (PROP-057
//! `##SHELL-XTASK-EMBED`, `##SHELL-RELATIVE`).
//!
//! A shell arrives as a directory of opaque, content-hashed files. Which
//! of them is the route template, what marker inside it is the island's
//! place, and which base path the addresses were built for are facts the
//! BUILD knows and the server cannot recover by looking. So the build
//! writes them down, and the server reads them — rather than guessing
//! from file names, which is the same class of mistake as parsing a
//! version out of a path.
//!
//! The index travels inside the shell, not beside it, because a shell
//! downloaded from a release asset and a shell compiled into the binary
//! have to answer the same questions and there is only one place both of
//! them are.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-XTASK-EMBED");

use std::path::Path;

use crate::error::{ShellError, ShellResult};

/// The file the index lives in, at the root of the shell.
pub const INDEX_FILE: &str = "shell.json";

/// The route template, at the root of the shell beside the index.
pub const PAGE_TEMPLATE: &str = "page-template.html";

/// The index's own shape version. It moves when this struct moves.
pub const INDEX_SCHEMA: u32 = 1;

/// What one shell says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// The index's shape version.
    pub schema: u32,
    /// The web package this shell was built from, `<group>/<name>`.
    pub package: String,
    /// That package's version.
    pub version: String,
    /// The base path the addresses inside the template were built for.
    /// A reader mounted elsewhere repoints them; it can only do that
    /// because the build said which prefix is its own.
    pub base: String,
    /// The exact string in the template the island replaces.
    pub island_marker: String,
    /// How many files the shell carries, for a report a person reads.
    pub files: u32,
    /// The digest `cargo xtask embed-doc-shell` measured when it wrote
    /// this shell — a CLAIM, checked against a fresh measurement of the
    /// bytes by `vibe doc shell status`.
    pub sha256: String,
}

type WireIndex = vibe_wire::generated::doc_shell_index::DocShellIndex;

impl From<WireIndex> for Index {
    fn from(index: WireIndex) -> Self {
        Self {
            schema: index.schema,
            package: index.package,
            version: index.version,
            base: index.base,
            island_marker: index.island_marker,
            files: index.files,
            sha256: index.sha256,
        }
    }
}

impl From<&Index> for WireIndex {
    fn from(index: &Index) -> Self {
        Self {
            schema: index.schema,
            package: index.package.clone(),
            version: index.version.clone(),
            base: index.base.clone(),
            island_marker: index.island_marker.clone(),
            files: index.files,
            sha256: index.sha256.clone(),
        }
    }
}

impl Index {
    /// Read an index from its JSON bytes.
    ///
    /// ```
    /// use vibe_doc_shell::index::Index;
    ///
    /// let json = r#"{"schema":1,"package":"org.vibevm.doc/web","version":"1.0.0",
    ///   "base":"/doc/","island_marker":"<!--vibe-doc-island-->","files":3,
    ///   "sha256":"00"}"#;
    /// let index = Index::parse(json, std::path::Path::new("shell/shell.json")).unwrap();
    /// assert_eq!(index.base, "/doc/");
    /// ```
    pub fn parse(text: &str, path: &Path) -> ShellResult<Index> {
        let index: WireIndex = serde_json::from_str(text).map_err(|error| ShellError::Index {
            path: path.to_path_buf(),
            message: format!("is not a readable index: {error}"),
        })?;
        if index.schema != INDEX_SCHEMA {
            return Err(ShellError::Index {
                path: path.to_path_buf(),
                message: format!(
                    "is written to index schema {} and this reader knows {INDEX_SCHEMA}",
                    index.schema
                ),
            });
        }
        Ok(index.into())
    }

    /// Render an index as the bytes the build writes.
    ///
    /// The type holds strings and integers only, so there is no
    /// serialisable state that can fail here; a fallible signature would
    /// push an impossible arm onto every caller.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(&WireIndex::from(self)).unwrap_or_default();
        text.push('\n');
        text
    }

    /// The index the bare fallback shell answers with.
    ///
    /// It names no package and no version, because the fallback is not a
    /// build of anything: it is this crate's own bytes, and saying
    /// otherwise would put a coordinate in a report that nothing can be
    /// fetched from.
    pub fn fallback() -> Index {
        Index {
            schema: INDEX_SCHEMA,
            package: String::new(),
            version: String::new(),
            base: crate::DEFAULT_BASE.to_string(),
            island_marker: crate::ISLAND_MARKER.to_string(),
            files: 0,
            sha256: String::new(),
        }
    }
}
