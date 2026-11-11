//! The registry error layer (Class F): every variant cites its
//! violated REQ and a fix surface (PROP-002 §2.3's failure
//! discriminator).

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;
use vibe_core::Group;

use crate::git_backend::GitError;

/// Failure surface of registry resolution, discriminated so the
/// multi-registry walk can route on it — `UnknownPackage` falls through
/// to the next registry, anything else halts (PROP-002 §2.3.1):
///
/// ```
/// use vibe_core::Group;
/// use vibe_registry::RegistryError;
///
/// let err = RegistryError::UnknownPackage {
///     group: Group::parse("org.vibevm").unwrap(),
///     name: "nope".to_string(),
/// };
/// assert_eq!(
///     err.to_string(),
///     "package `org.vibevm/nope` is not in the registry \
///      (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
///       fix: check the spelling or add a [[registry]] that carries it)",
/// );
/// ```
#[derive(Debug, Error)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
)]
pub enum RegistryError {
    /// The template layer refused (PROP-075 `#TEMPLATE-DOMAIN-SEAM`).
    /// Boxed so one rare variant does not widen every `RegistryError` on
    /// the stack.
    #[error(transparent)]
    Template(Box<crate::templates::TemplateError>),
    #[error(transparent)]
    SourcePolicy(#[from] vibe_core::manifest::SourcePolicyError),
    #[error(transparent)]
    Index(crate::IndexError),
    #[error(transparent)]
    ContactRefused(#[from] crate::ContactRefusal),
    #[error(transparent)]
    Configuration(#[from] vibe_core::manifest::PackageConfigurationError),

    #[error(
        "registry root `{0}` does not exist or is not a directory \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model; \
          fix: check the [[registry]] url or pass --registry <dir>)"
    )]
    MissingRoot(PathBuf),

    #[error(
        "package `{group}/{name}` is not in the registry \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check the spelling or add a [[registry]] that carries it)"
    )]
    UnknownPackage { group: Group, name: String },

    /// A remap row — or the active `unmatched = "deny"` policy — refused
    /// this registry before any connection (`PROP-066#row`). Named here
    /// rather than folded into `UnknownPackage`, because "nobody may fetch
    /// from there" and "nothing is there" are different facts and the
    /// reader acts differently on each.
    #[error(
        "remap_denied: reading registry `{registry}` at `{url}` is refused by {row}: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#row; \
          fix: drop or narrow that remap row, or point it at a clone this host may \
          read — `vibe remap explain {url}` prints the whole chain)"
    )]
    RemapDenied {
        registry: String,
        url: String,
        row: String,
        reason: String,
    },

    #[error(
        "no version of `{group}/{name}` matches `{req}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: relax the version requirement or run `vibe registry sync`)"
    )]
    NoMatchingVersion {
        group: Group,
        name: String,
        req: String,
    },

    /// Every candidate version that matched the requirement was one
    /// this reader must quarantine (PROP-044 §4.5): its record's
    /// `must_understand` names a reader capability this build lacks,
    /// so the refusal surfaces here, at the point of application —
    /// the reader never silently acts on a record it cannot honour
    /// (B-080). The payload is boxed ([`AllVersionsUnusableDetail`])
    /// so the enum stays under the result-large-err budget every
    /// `Result<_, RegistryError>` fn pays — the same shape
    /// [`RegistryError::StoreEntryMismatch`] already pays.
    #[error(
        "{detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-044#machinery; \
          fix: update vibe to a build that understands them, or ask for a version \
          this build can act on)"
    )]
    AllVersionsUnusable {
        detail: Box<AllVersionsUnusableDetail>,
    },

    /// A pkgref reached registry resolution without a `group`. A registry
    /// resolves by `(group, name)` identity (PROP-008 §2.2); a bare short
    /// name must be qualified at the CLI boundary first.
    #[error(
        "package reference `{0}` is not group-qualified — registry resolution needs \
         `<group>/<name>` (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model; \
         fix: qualify the reference as `<group>/<name>`)"
    )]
    UnqualifiedPkgref(String),

    #[error(
        "registry entry at `{path}` has an invalid directory name `{name}` — expected \
         `v<semver>` (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#layout; \
         fix: rename the version directory to `v<major>.<minor>.<patch>`)"
    )]
    BadVersionDir { path: PathBuf, name: String },

    /// A local `[[registry]]` url (`file://` / bare path per `url_is_local`)
    /// did not resolve to a filesystem path. Local registries are served by
    /// `LocalRegistry` straight off the filesystem (PROP-002 §2.2.2), so a
    /// malformed local url is caught at construction, not mid-walk.
    #[error(
        "`{url}` is a local `[[registry]]` url that does not resolve to a filesystem path \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model; \
          fix: use a `file:///<path>` URL or a bare path pointing at a directory laid out \
          `<group>/<name>/v<version>/`)"
    )]
    BadLocalRegistryUrl { url: String },

    /// A monorepo registry declared with `auth = "token-env"`.
    #[error(
        "registry `{registry}` is a monorepo with `auth = \"token-env\"`, which a single-repository \
         clone does not inject (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type; \
         fix: use an SSH url with `auth = \"ssh\"` (keys from ~/.ssh) or `auth = \"credential-helper\"`)"
    )]
    MonorepoTokenAuth { registry: String },

    /// A `[[registry]]` whose declared `type` contradicts its url.
    #[error(transparent)]
    RegistryType {
        #[from]
        source: vibe_core::manifest::RegistryTypeError,
    },

    #[error(transparent)]
    Core(#[from] vibe_core::Error),

    #[error(
        "git operation failed \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: act on the wrapped git error): {0}"
    )]
    Git(GitError),

    /// A registry git operation failed, stated as the errand it was on
    /// rather than as the argv it ran (`PROP-002
    /// #failure-discriminator`). The detail names the operation in
    /// words, the registry and its real url, the package, the
    /// translated cause with its fix, and the registries this profile
    /// allows; git's own stderr tail follows BELOW that explanation,
    /// which is why the trailer sits between the two here instead of at
    /// the end. Boxed ([`crate::GitFailureDetail`]) so one wide, rare
    /// variant does not widen every `Result<_, RegistryError>` — the
    /// shape [`RegistryError::AllVersionsUnusable`] already pays.
    #[error(
        "{} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: act on the cause and fix named above){}",
        detail.explanation(),
        detail.git_said()
    )]
    GitOperation {
        detail: Box<crate::GitFailureDetail>,
    },

    #[error(
        "could not determine the user home directory; set HOME (or USERPROFILE on Windows), or \
         pass an explicit cache root \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#cache; \
          fix: set HOME / USERPROFILE or VIBE_REGISTRY_CACHE)"
    )]
    NoHomeDir,

    #[error(
        "registry meta file at `{path}` is malformed \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: correct or regenerate the file at that path): {reason}"
    )]
    MalformedMeta { path: PathBuf, reason: String },

    #[error(
        "auth declaration for `{registry}` at {endpoint}: {reason}; violates REQ spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth; fix surface: selected-source auth/token_env declaration"
    )]
    AuthDeclaration {
        registry: String,
        endpoint: String,
        reason: String,
    },

    /// Registry is declared `auth = "token-env"` (PROP-002 §2.2.1) but
    /// the resolved env-var is empty / unset. Surfaces before any git
    /// invocation so the operator gets an actionable hint pointing at
    /// the env-var to set, instead of a generic 401 from the host.
    #[error(
        "registry `{registry}` declares `auth = \"token-env\"` but env-var `{env_var}` is empty or unset; \
         set it to a personal access token with read access to the registry org \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth; \
          fix: export {env_var})"
    )]
    MissingToken { registry: String, env_var: String },

    /// Aggregated walk-failure: every configured registry was tried,
    /// none had a satisfying answer, and at least one walked-past
    /// 401 / 403 (auth=none) needs surfacing so the operator sees
    /// per-registry status. `summary` is the pre-formatted
    /// multi-line block that `Display` renders verbatim;
    /// `attempts` carries the same information in structured form
    /// so `vibe-cli`'s install-error JSON envelope can ship a
    /// machine-readable per-registry array without the consumer
    /// having to parse prose. Returned only when at least one
    /// registry was walked; the no-registries-at-all path still
    /// returns the simpler `UnknownPackage` variant for back-compat
    /// with downstream consumers that match on it.
    #[error(
        "package `{group}/{name}` not found in any configured registry \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check the package name and `vibe registry list`).\nTried:\n{summary}"
    )]
    PackageNotFoundEverywhere {
        group: Group,
        name: String,
        summary: String,
        attempts: Vec<crate::multi_registry_resolver::RegistryWalkAttempt>,
    },

    /// A registry was named explicitly — `--registry <name>`, a lockfile pin
    /// — but the effective source policy excludes its hosting, so it is not
    /// in play (PROP-073 `#SOURCE-POLICY`). The declaration itself is
    /// untouched; `exclude_hosting` switches the registry off rather than
    /// removing it, and the resolution walk skips it without ever opening
    /// it. Only a by-name selection reaches this error.
    #[error(
        "registry `{registry}` is disabled by the effective source policy, which excludes \
         `{selector}` hosting \
         (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; \
          fix: name a registry the policy admits, or drop `{selector}` from `exclude_hosting`)"
    )]
    RegistryDisabledByPolicy { registry: String, selector: String },

    #[error(
        "I/O error on `{path}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check the path's existence and permissions)"
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An `in-place` materialization (PROP-022 §2.4) was requested against a
    /// registry with no git backend — the local-directory registry
    /// (`--registry <path>`, the M0 shape). In-place needs a real git source
    /// to clone and incrementally update; there is nothing to clone from a
    /// directory tree.
    #[error(
        "package `{group}/{name}` declares in-place materialization but resolves through a \
         local-directory registry with no git backend \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place; \
          fix: serve it from a git `[[registry]]`, or drop `materialization = \"in-place\"`)"
    )]
    InPlaceUnsupported { group: Group, name: String },

    #[error(
        "prepared in-place source at `{path}` cannot be applied: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION; fix: prepare and admit the actual source again)"
    )]
    PreparedInPlace { path: PathBuf, reason: String },

    /// A store entry that was about to become the materialisation source
    /// no longer hashes to the lockfile pin (PROP-010 §2.7 — a mismatch is
    /// named, never swallowed). Our code never rewrites an entry, so bytes
    /// that disagree with the pin were changed outside vibevm; the ordinary
    /// path does not re-hash the store, this fires only where a pin exists
    /// and the entry is actually read as the source. The payload is boxed
    /// ([`StoreEntryMismatchDetail`]) so the enum stays under the
    /// result-large-err budget every `Result<_, RegistryError>` fn pays.
    #[error(
        "{detail} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#layout; \
          fix: run `vibe cache check --repair`, or delete that entry directory \
          so the next fetch re-downloads it)"
    )]
    StoreEntryMismatch {
        detail: Box<StoreEntryMismatchDetail>,
    },

    /// A held lock row was replayed from its locked source, but the bytes
    /// that source produced no longer hash to the row's `content_hash`
    /// (PROP-073 `#LOCKED-SOURCE-LAW`). Typed so a caller that owns its own
    /// refusal vocabulary (`vibe reinstall --force`) can name it exactly.
    #[error(
        "locked source: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW; \
          fix: restore the locked source or explicitly update its version)"
    )]
    LockedContentMismatch {
        detail: Box<LockedContentMismatchDetail>,
    },

    /// An offline resolution reached the end of its local sources —
    /// the machine store and every local (`file://`) registry — without
    /// the requested package (PROP-010 §2.5, `OFFLINE-HARD-ERROR`): a
    /// hard error, never a silent degrade, naming the package and the
    /// recovery recipes.
    #[error(
        "`{group}/{name}@{req}` is not resolvable offline — it is in no local \
         (`file://`) registry and not in the machine store \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#offline; \
          fix: run the install once online to warm the store, pre-warm it with \
          `vibe cache add {group}/{name}`, or export a `file://` mirror with \
          `vibe registry vendor`)"
    )]
    OfflinePackageUnavailable {
        group: Group,
        name: String,
        req: String,
    },
}

/// The structured payload of
/// [`RegistryError::AllVersionsUnusable`] — which package, which
/// requirement, the best version the pick had to skip, and the reader
/// capabilities it lacks. Renders as the refusal's own sentence so
/// the variant's grammar only adds the violated spec and the fix.
#[derive(Debug)]
pub struct AllVersionsUnusableDetail {
    pub group: Group,
    pub name: String,
    pub req: String,
    pub best: semver::Version,
    pub missing: Vec<String>,
}

impl std::fmt::Display for AllVersionsUnusableDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "every version of `{}/{}` matching `{}` was skipped — this reader \
             does not understand capabilities their records declare; best skipped \
             `{}` needs {:?}",
            self.group, self.name, self.req, self.best, self.missing
        )
    }
}

/// The structured payload of
/// [`RegistryError::StoreEntryMismatch`] — which entry, which pin,
/// what the entry actually hashes to. Renders as the mismatch's own
/// sentence so the variant's grammar only adds the violated spec and
/// the fix.
#[derive(Debug)]
pub struct StoreEntryMismatchDetail {
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    pub path: PathBuf,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for StoreEntryMismatchDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let identity = format!("{}/{}@{}", self.group, self.name, self.version);
        write!(
            f,
            "store entry for `{identity}` at `{}` hashes to {}, but the \
             lockfile pins {}",
            self.path.display(),
            self.actual,
            self.expected,
        )
    }
}

/// The structured payload of [`RegistryError::LockedContentMismatch`]:
/// the held coordinate, the hash its lock row pins and the hash its
/// locked source now produces.
#[derive(Debug)]
pub struct LockedContentMismatchDetail {
    pub group: String,
    pub name: String,
    pub version: semver::Version,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for LockedContentMismatchDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "acquired content of `{}/{}@{}` hashes to {}, but the held lock row pins {}",
            self.group, self.name, self.version, self.actual, self.expected,
        )
    }
}

impl From<GitError> for RegistryError {
    fn from(error: GitError) -> Self {
        match error {
            GitError::ContactRefused(refusal) => Self::ContactRefused(refusal),
            other => Self::Git(other),
        }
    }
}
impl From<crate::IndexError> for RegistryError {
    fn from(error: crate::IndexError) -> Self {
        match error {
            crate::IndexError::ContactRefused(refusal) => Self::ContactRefused(refusal),
            other => Self::Index(other),
        }
    }
}
