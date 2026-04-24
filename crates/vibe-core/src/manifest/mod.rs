//! Manifest schemas used throughout vibevm.
//!
//! Three manifests exist:
//! - [`ProjectManifest`] — `vibe.toml` at a project's root. Schema:
//!   `VIBEVM-SPEC.md` §7.5.
//! - [`PackageManifest`] — `vibe-package.toml` inside a package directory.
//!   Schema: `VIBEVM-SPEC.md` §7.3.
//! - [`Lockfile`] — `vibe.lock` at a project's root. Schema: `VIBEVM-SPEC.md`
//!   §7.4.

mod lockfile;
mod package;
mod project;

pub use lockfile::{CURRENT_SCHEMA_VERSION, Lockfile, LockedPackage, LockfileMeta};
pub use package::{
    BootSnippet, Compatibility, ConflictsList, Obsoletes, PackageDependencies, PackageManifest,
    PackageMeta, Provides, Requires, RequiresAny, WritesSection,
};
pub use project::{
    ActiveSection, DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_REF, DEFAULT_REGISTRY_URL, LlmSection,
    MirrorSection, NamingConvention, OverrideSection, ProjectManifest, ProjectSection,
    RegistrySection,
};

use std::fs;
use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

use crate::error::{Error, Result};

pub(crate) fn read_toml<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str::<T>(&text).map_err(|source| Error::ParseToml {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn write_toml<T, P>(path: P, value: &T) -> Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let rendered = toml::to_string_pretty(value)?;
    fs::write(path, rendered).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}
