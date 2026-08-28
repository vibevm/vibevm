//! The neutral construction options — the ONE argument shape the
//! package-source builder reads, projected by each surface from its own
//! grammar with no normalisation and no validation reordering.

use std::path::PathBuf;

/// The registry / solver / source-preference inputs of
/// [`build_install_resolver`](crate::build_install_resolver), carrying
/// exactly the fields that builder reads.
///
/// `Default` is the **hosted posture**: no flags, the resolvo default, the
/// public auth walk (`auth_required = false`), and local / project /
/// embedded discovery all enabled — the configuration a hosted surface with
/// no argument grammar of its own runs with. The CLI projects its
/// `InstallArgs` onto this value as a pure field copy; a later MCP adapter
/// passes the default unchanged.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_package_source::PackageSourceOptions;
///
/// // The hosted posture: every flag off, every discovery lane on.
/// let hosted = PackageSourceOptions::default();
/// assert!(hosted.registry.is_none());
/// assert!(!hosted.auth_required);
/// assert!(!hosted.no_default_registry);
///
/// // A surface's own grammar arrives as a plain field projection.
/// let explicit = PackageSourceOptions {
///     registry: Some(PathBuf::from("tests/fixtures/registry")),
///     ..Default::default()
/// };
/// assert_eq!(explicit.registry, Some(PathBuf::from("tests/fixtures/registry")));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageSourceOptions {
    /// An explicit local-directory registry path (`--registry <path>`),
    /// overriding the declared/embedded discovery entirely.
    pub registry: Option<PathBuf>,
    /// The dependency solver cell name (`resolvo` default; `naive` / `sat`
    /// selectable fallbacks). `None` keeps the built-in default.
    pub solver: Option<String>,
    /// Strict authentication gate: a 401/403 from an `auth = "none"`
    /// (public) registry halts the walk instead of falling through to the
    /// next registry.
    pub auth_required: bool,
    /// Prefer the embedded registry over the declared walk (an explicit
    /// affirmation of the default; paired with `no_prefer_embedded` as a
    /// mutual-exclusivity guard).
    pub prefer_embedded: bool,
    /// Consult the declared `[[registry]]` walk before the embedded
    /// registry on a coordinate clash.
    pub no_prefer_embedded: bool,
    /// Ignore the ambient embedded registry entirely for this run.
    pub no_default_registry: bool,
    /// Short-circuit version enumeration at the embedded registry for any
    /// coordinate it serves — the declared walk is consulted only for what
    /// the embedded registry lacks.
    pub embedded_short_circuit: bool,
    /// Explicitly opt in to project-local packages winning a clash inside
    /// the local-registry family (an affirmation of the default).
    pub prefer_local: bool,
    /// Ignore the project-local `packages/` directory for this run.
    pub no_prefer_local: bool,
    /// Whether the surface's grammar carried a direct git-source
    /// declaration for this run (the M1.15 `--git` flag). The manifest's
    /// own git-source entries are read by the builder regardless.
    pub has_git_source_flag: bool,
}
