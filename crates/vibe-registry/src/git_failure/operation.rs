//! The errand a failed git invocation was on, in words.
//!
//! Not an argv: `git fetch --progress --no-tags -- origin
//! +refs/tags/v26.13.0:refs/tags/v26.13.0` is the mechanism, and reading
//! it is how an operator ends up believing vibe built the command wrong.
//! What they need first is *what vibe was trying to do* — listing the
//! versions of a package, fetching one version, capturing a registry's
//! mirror — which only the caller knows and therefore only the caller
//! can state.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use std::fmt;

use vibe_core::Group;

/// One registry git errand, with the identity it is about.
///
/// ```
/// use vibe_core::Group;
/// use vibe_registry::git_failure::GitOperation;
///
/// let group = Group::parse("org.example").unwrap();
/// let listing = GitOperation::list_versions(&group, "thing");
/// assert_eq!(listing.to_string(), "listing the versions of `org.example/thing`");
/// let fetching = GitOperation::fetch_package(&group, "thing", "v1.2.3");
/// assert_eq!(fetching.to_string(), "fetching `org.example/thing` version `1.2.3`");
/// assert_eq!(fetching.package(), Some("org.example/thing".to_string()));
/// assert_eq!(fetching.version(), Some("1.2.3"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitOperation {
    /// Enumerating what versions a package has — `git ls-remote --tags`.
    ListVersions { package: String },
    /// Bringing one version of a package onto this machine.
    FetchVersion { package: String, version: String },
    /// Bringing one package onto this machine at a ref that is not a
    /// version tag (a branch, a commit, an operator-declared `tag`).
    FetchRef { package: String, reference: String },
    /// Reading a single file out of a package without a clone.
    ReadFile {
        package: String,
        reference: String,
        path: String,
    },
    /// Cloning or refreshing a whole-repository (`monorepo`) registry.
    CaptureMirror { registry: String },
}

impl GitOperation {
    /// `<group>/<name>`, the spelling every other surface uses.
    fn identity(group: &Group, name: &str) -> String {
        format!("{group}/{name}")
    }

    pub fn list_versions(group: &Group, name: &str) -> Self {
        GitOperation::ListVersions {
            package: Self::identity(group, name),
        }
    }

    /// The fetch errand for `reference`: a `v<semver>` tag is the
    /// version it names, anything else stays a ref — an operator who
    /// declared `branch = "main"` must not read back "version `main`".
    pub fn fetch_package(group: &Group, name: &str, reference: &str) -> Self {
        let package = Self::identity(group, name);
        match version_of(reference) {
            Some(version) => GitOperation::FetchVersion { package, version },
            None => GitOperation::FetchRef {
                package,
                reference: reference.to_string(),
            },
        }
    }

    pub fn read_file(group: &Group, name: &str, reference: &str, path: &str) -> Self {
        GitOperation::ReadFile {
            package: Self::identity(group, name),
            reference: reference.to_string(),
            path: path.to_string(),
        }
    }

    pub fn capture_mirror(registry: &str) -> Self {
        GitOperation::CaptureMirror {
            registry: registry.to_string(),
        }
    }

    /// The package this errand is about, when it is about one.
    pub fn package(&self) -> Option<String> {
        match self {
            GitOperation::ListVersions { package }
            | GitOperation::FetchVersion { package, .. }
            | GitOperation::FetchRef { package, .. }
            | GitOperation::ReadFile { package, .. } => Some(package.clone()),
            GitOperation::CaptureMirror { .. } => None,
        }
    }

    /// The version this errand is about, when it names one.
    pub fn version(&self) -> Option<&str> {
        match self {
            GitOperation::FetchVersion { version, .. } => Some(version),
            _ => None,
        }
    }
}

/// The version a `v<semver>` tag names — `None` for any other ref.
fn version_of(reference: &str) -> Option<String> {
    let stripped = reference.strip_prefix('v')?;
    semver::Version::parse(stripped)
        .ok()
        .map(|_| stripped.into())
}

impl fmt::Display for GitOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitOperation::ListVersions { package } => {
                write!(f, "listing the versions of `{package}`")
            }
            GitOperation::FetchVersion { package, version } => {
                write!(f, "fetching `{package}` version `{version}`")
            }
            GitOperation::FetchRef { package, reference } => {
                write!(f, "fetching `{package}` at ref `{reference}`")
            }
            GitOperation::ReadFile {
                package,
                reference,
                path,
            } => write!(f, "reading `{path}` of `{package}` at ref `{reference}`"),
            GitOperation::CaptureMirror { registry } => {
                write!(f, "capturing the mirror of registry `{registry}`")
            }
        }
    }
}
