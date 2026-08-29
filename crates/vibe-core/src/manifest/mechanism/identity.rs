//! The mechanism *addressing* vocabulary — the values a manifest uses to name
//! what services a target.
//!
//! - [`MechanismRole`] — the four provider roles.
//! - [`MechanismKey`] — the logical `<role>:<portable-name>` capability a
//!   target selects.
//! - [`ProviderPin`] — the exact `<group>/<package>#<id>` provider identity a
//!   pin or a host route resolves to.
//!
//! A key is a capability and a pin is an identity; keeping the two apart is
//! what stops an installed package from seizing a logical role by name.
//! Parsing is the only constructor for each, so an illegal value is
//! unrepresentable rather than merely rejected downstream.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::fmt;
use std::str::FromStr;

use crate::HostOwner;

const ONE_MACHINE: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";

const MECHANISM_ROLES: [MechanismRole; 4] = [
    MechanismRole::Build,
    MechanismRole::Package,
    MechanismRole::Deploy,
    MechanismRole::Acquire,
];

/// The role a mechanism provider services. `Acquire` is a provider role
/// reserved for a later artifact source; it is not accepted by the build,
/// package, or deploy target arrays.
///
/// ```
/// use vibe_core::manifest::MechanismRole;
///
/// assert_eq!(MechanismRole::Build.as_str(), "build");
/// assert_eq!("acquire".parse(), Ok(MechanismRole::Acquire));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MechanismRole {
    Build,
    Package,
    Deploy,
    Acquire,
}

impl MechanismRole {
    /// The exact lowercase wire spelling.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Package => "package",
            Self::Deploy => "deploy",
            Self::Acquire => "acquire",
        }
    }

    /// Every legal role, in declaration order.
    pub const fn all() -> [MechanismRole; 4] {
        MECHANISM_ROLES
    }
}

impl fmt::Display for MechanismRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MechanismRole {
    type Err = MechanismRoleParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        MECHANISM_ROLES
            .into_iter()
            .find(|role| role.as_str() == input)
            .ok_or_else(|| MechanismRoleParseError {
                input: input.to_owned(),
            })
    }
}

/// A value that is not one of the four mechanism roles.
///
/// ```
/// use vibe_core::manifest::MechanismRole;
///
/// let error = "test".parse::<MechanismRole>().unwrap_err();
/// assert_eq!(error.input(), "test");
/// assert!(error.to_string().contains("build, package, deploy, acquire"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanismRoleParseError {
    input: String,
}

impl fmt::Display for MechanismRoleParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid mechanism role `{}`; expected one of: build, package, deploy, acquire ({ONE_MACHINE})",
            self.input,
        )
    }
}

impl std::error::Error for MechanismRoleParseError {}

impl MechanismRoleParseError {
    /// Return the exact value that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

/// A logical mechanism key — `<role>:<portable-name>`, the capability
/// vocabulary a target selects (e.g. `build:cargo`, `deploy:vibe-bin`). A
/// key is a capability, never a package or provider identity.
///
/// ```
/// use vibe_core::manifest::{MechanismKey, MechanismRole};
///
/// let key: MechanismKey = "build:cargo".parse().unwrap();
/// assert_eq!(key.role(), MechanismRole::Build);
/// assert_eq!(key.name(), "cargo");
/// assert_eq!(key.to_string(), "build:cargo");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MechanismKey {
    role: MechanismRole,
    name: String,
}

impl MechanismKey {
    /// The key's role family.
    pub fn role(&self) -> MechanismRole {
        self.role
    }

    /// The key's portable mechanism name (the part after `:`).
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for MechanismKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.role, self.name)
    }
}

impl FromStr for MechanismKey {
    type Err = MechanismKeyParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let Some((role, name)) = input.split_once(':') else {
            return Err(MechanismKeyParseError::shape(input));
        };
        if role.split_once(':').is_some() || name.contains(':') {
            return Err(MechanismKeyParseError::shape(input));
        }
        let role = role
            .parse::<MechanismRole>()
            .map_err(|_| MechanismKeyParseError::shape(input))?;
        if !is_portable_token(name) {
            return Err(MechanismKeyParseError::shape(input));
        }
        Ok(Self {
            role,
            name: name.to_owned(),
        })
    }
}

/// A value that is not a `<role>:<portable-name>` mechanism key.
///
/// ```
/// use vibe_core::manifest::MechanismKey;
///
/// let error = "cargo".parse::<MechanismKey>().unwrap_err();
/// assert_eq!(error.input(), "cargo");
/// assert!(error.to_string().contains("`<role>:<portable-name>`"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanismKeyParseError {
    input: String,
}

impl MechanismKeyParseError {
    fn shape(input: &str) -> Self {
        Self {
            input: input.to_owned(),
        }
    }
}

impl fmt::Display for MechanismKeyParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid mechanism key `{}`; expected `<role>:<portable-name>` with role one of build, package, deploy, acquire and a nonempty portable name ({ONE_MACHINE})",
            self.input,
        )
    }
}

impl std::error::Error for MechanismKeyParseError {}

impl MechanismKeyParseError {
    /// Return the exact value that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

pub use crate::host_owner::HOST_OWNER;

/// Who owns an exact provider — the structural half of a [`ProviderPin`], and
/// the same two-kind split the landed R2 `HostIdentity` law already draws
/// (`vibe-extension-registry`'s `HostIdentity`):
///
/// - a `[package]`, and a **grouped** `[project]`, own a real coordinate
///   `<group>/<name>` — `HostIdentity::Coordinate`;
/// - only an **ungrouped** `[project]` owns `__host__/<project-name>`, because
///   it has no group to form a coordinate from — `HostIdentity::UngroupedProject`.
///
/// Callers (state keys, registry projection, attribution) tell the kinds apart
/// structurally rather than by re-parsing a string.
///
/// ```
/// use vibe_core::manifest::{ProviderOwner, ProviderPin};
///
/// let package: ProviderPin = "org.example/build-tools#cargo-v2".parse().unwrap();
/// let host: ProviderPin = "__host__/demo#cargo-v2".parse().unwrap();
/// assert!(matches!(package.owner(), ProviderOwner::Package { .. }));
/// assert!(matches!(host.owner(), ProviderOwner::Host { .. }));
/// assert_ne!(package, host);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProviderOwner {
    /// A real coordinate `<group>/<name>`: a package, or a grouped project.
    Package {
        group: crate::Group,
        package: crate::PackageName,
    },
    /// An ungrouped project host. Opaque — the project name is never parsed
    /// as a [`crate::PackageName`] and the head is never parsed as a
    /// [`crate::Group`] (SPEC-DEBT-LIFECYCLE §8.3). `project` holds the
    /// authored name verbatim; the one host-owner codec ([`HostOwner`])
    /// turns it into the canonical `__host__/<segment>` spelling and back.
    Host { project: String },
}

impl fmt::Display for ProviderOwner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package { group, package } => write!(formatter, "{group}/{package}"),
            Self::Host { project } => write!(formatter, "{}", HostOwner::new(project.clone())),
        }
    }
}

/// An exact provider identity — `<owner>#<id>`, where the owner is either a
/// package coordinate `<group>/<package>` or the project host
/// `__host__/<project-name>`. Never a short id, never PackageRef version
/// syntax. [`Display`](fmt::Display) is canonical and round-trips.
///
/// ```
/// use vibe_core::manifest::ProviderPin;
///
/// let pin: ProviderPin = "org.example/build-tools#cargo-v2".parse().unwrap();
/// assert_eq!(pin.id(), "cargo-v2");
/// assert_eq!(pin.to_string(), "org.example/build-tools#cargo-v2");
/// assert!("cargo-v2".parse::<ProviderPin>().is_err());
///
/// let host: ProviderPin = "__host__/my-project#cargo-v2".parse().unwrap();
/// assert_eq!(host.to_string(), "__host__/my-project#cargo-v2");
/// assert_eq!(host.host_project(), Some("my-project"));
/// assert!(host.group().is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProviderPin {
    owner: ProviderOwner,
    id: String,
}

impl ProviderPin {
    /// Parse and validate an exact provider pin.
    pub fn parse(input: &str) -> Result<Self, ProviderPinParseError> {
        let refuse = |reason: &str| ProviderPinParseError {
            input: input.to_owned(),
            reason: reason.to_owned(),
        };
        if input.chars().any(char::is_whitespace) {
            return Err(refuse("provider identities carry no whitespace"));
        }
        if input.contains('@') {
            return Err(refuse(
                "a version constraint is PackageRef syntax and is not part of a provider identity",
            ));
        }
        let Some((coordinate, id)) = input.split_once('#') else {
            if input.contains('/') {
                return Err(refuse("missing the `#<id>` part"));
            }
            return Err(refuse(
                "a short id is not a provider identity; write the full `<group>/<package>#<id>` or `__host__/<project-name>#<id>`",
            ));
        };
        if id.is_empty() {
            return Err(refuse("the `#<id>` part is empty"));
        }
        if !is_portable_token(id) {
            return Err(refuse(
                "the `#<id>` part is not a portable token (lowercase alphanumerics, `-`, `.`)",
            ));
        }
        if coordinate.contains(':') {
            return Err(refuse(
                "a kind prefix is PackageRef syntax and is not part of a provider identity",
            ));
        }
        let Some((head, tail)) = coordinate.split_once('/') else {
            return Err(refuse(
                "expected `<group>/<package>` or `__host__/<project-name>` before `#`",
            ));
        };
        // The host branch is opaque on purpose: an ungrouped project has no
        // package coordinate, so nothing here may reach for `Group` or
        // `PackageName` (SPEC-DEBT-LIFECYCLE §8.3). The segment goes through
        // the one host-owner codec, which is why splitting at the first `#`
        // above is unambiguous: a project name can never spell a literal `#`.
        let owner = if head == HOST_OWNER {
            let owner = HostOwner::parse_segment(tail).map_err(|fault| {
                refuse(&format!(
                    "the `__host__/<segment>` part is not a canonical host identity: {}",
                    fault.reason()
                ))
            })?;
            ProviderOwner::Host {
                project: owner.project().to_owned(),
            }
        } else {
            let group = head.parse::<crate::Group>().map_err(|_| {
                refuse("`<group>` is not a valid reverse-FQDN group (lowercase LDH labels)")
            })?;
            let package = tail
                .parse::<crate::PackageName>()
                .map_err(|_| refuse("`<package>` is not a valid kebab-case package name"))?;
            ProviderOwner::Package { group, package }
        };
        Ok(Self {
            owner,
            id: id.to_owned(),
        })
    }

    /// Who owns this provider — match on it rather than re-parsing the text.
    pub fn owner(&self) -> &ProviderOwner {
        &self.owner
    }

    /// The provider's declaring group, or `None` for a project host.
    pub fn group(&self) -> Option<&crate::Group> {
        match &self.owner {
            ProviderOwner::Package { group, .. } => Some(group),
            ProviderOwner::Host { .. } => None,
        }
    }

    /// The provider's declaring package name, or `None` for a project host.
    pub fn package(&self) -> Option<&crate::PackageName> {
        match &self.owner {
            ProviderOwner::Package { package, .. } => Some(package),
            ProviderOwner::Host { .. } => None,
        }
    }

    /// The owning project's name, or `None` for a package owner.
    pub fn host_project(&self) -> Option<&str> {
        match &self.owner {
            ProviderOwner::Package { .. } => None,
            ProviderOwner::Host { project } => Some(project),
        }
    }

    /// The provider's portable id within its owner.
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for ProviderPin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}#{}", self.owner, self.id)
    }
}

impl FromStr for ProviderPin {
    type Err = ProviderPinParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Why a [`ProviderPin::parse`] failed.
///
/// ```
/// use vibe_core::manifest::ProviderPin;
///
/// let error = ProviderPin::parse("org.example/build-tools#x@1.0").unwrap_err();
/// assert_eq!(error.input(), "org.example/build-tools#x@1.0");
/// assert!(error.to_string().contains("version constraint"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPinParseError {
    input: String,
    reason: String,
}

impl fmt::Display for ProviderPinParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid provider identity `{}`: {} ({ONE_MACHINE})",
            self.input, self.reason,
        )
    }
}

impl std::error::Error for ProviderPinParseError {}

impl ProviderPinParseError {
    /// Return the exact value that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

/// A nonempty portable token: lowercase ASCII alphanumerics plus `-` and `.`,
/// starting and ending alphanumeric, with no `..` run. The shared id/name
/// vocabulary for mechanism keys, provider ids, artifact ids and kinds, and
/// profile names — the ONE grammar of the whole mechanism plane.
///
/// `pub` since the A1/A2 follow-up: `vibe-wire`'s record scalars cite this
/// exact law for the members the R8 record formats carry, and a second
/// byte-for-byte copy there was one drift away from two grammars of one
/// identity. Exporting the predicate makes the wire a DELEGATE of this one
/// authority; the function lends a `bool` and nothing else.
pub fn is_portable_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let edge_ok = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !edge_ok(bytes[0]) || !edge_ok(bytes[bytes.len() - 1]) {
        return false;
    }
    if !bytes
        .iter()
        .all(|byte| edge_ok(*byte) || *byte == b'-' || *byte == b'.')
    {
        return false;
    }
    !value.contains("..")
}
