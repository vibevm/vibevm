//! `[[mechanism]]` provider declarations and `[mechanisms]` host routes.
//!
//! A mechanism is a *provider* that can be selected for one role of the
//! build/package/deploy plane — the sibling of `[[extension]]` on the one
//! extension machine. It adds lookup, not a second scheduler: a mechanism is
//! inert until a target selects it by logical key or exact provider pin.
//!
//! This cell is pure manifest grammar and validation. Nothing here executes a
//! provider, resolves a route against the installed world, or writes any
//! artifact/destination state. The addressing vocabulary a declaration and a
//! route are written in lives next door, in [`identity`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

mod identity;
mod wire;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use indexmap::IndexMap;

use super::declarant_path::{declarant_path, declarant_path_error};
use super::extension::{ExtensionHandler, validate_handler_shape};

pub use identity::{
    HOST_OWNER, MechanismKey, MechanismKeyParseError, MechanismRole, MechanismRoleParseError,
    ProviderOwner, ProviderPin, ProviderPinParseError, is_portable_token,
};
pub(crate) use wire::{MechanismDeclWire, MechanismRoutesWire};

const ONE_MACHINE: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";

/// One `[[mechanism]]` provider declaration.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::manifest::{
///     ExtensionHandler, MechanismDecl, MechanismFreshness, MechanismRole,
/// };
///
/// let declaration = MechanismDecl {
///     id: "cargo-v2".into(),
///     role: MechanismRole::Build,
///     name: "cargo".into(),
///     handler: ExtensionHandler::Native { crate_dir: Some(PathBuf::from("crates/cargo-provider")), prebuilt: None },
///     protocol: 1,
///     config_schema: PathBuf::from("schemas/cargo-build-v1.jtd.json"),
///     freshness: MechanismFreshness::Provider,
/// };
/// assert!(declaration.validate().is_ok());
/// ```
///
/// The authored TOML form reaches this type through
/// [`Manifest::parse_str`](super::Manifest::parse_str):
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let manifest = Manifest::parse_str(concat!(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
///     "[[mechanism]]\nid = \"cargo-v2\"\nrole = \"build\"\nname = \"cargo\"\n",
///     "handler = { kind = \"native\", crate_dir = \"crates/cargo-provider\" }\n",
///     "protocol = 1\nconfig_schema = \"schemas/cargo-build-v1.jtd.json\"\n",
///     "freshness = \"provider\"\n",
/// )).unwrap();
/// assert_eq!(manifest.mechanism_decls.len(), 1);
/// assert_eq!(manifest.mechanism_decls[0].id, "cargo-v2");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanismDecl {
    pub id: String,
    pub role: MechanismRole,
    pub name: String,
    pub handler: ExtensionHandler,
    pub protocol: u32,
    pub config_schema: PathBuf,
    pub freshness: MechanismFreshness,
}

impl MechanismDecl {
    /// Revalidate a declaration constructed through Rust rather than TOML.
    pub fn validate(&self) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err(format!(
                "[[mechanism]] field `id` value `{}` is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) ({ONE_MACHINE})",
                self.id,
            ));
        }
        if !is_portable_token(&self.name) {
            return Err(format!(
                "[[mechanism]] `{}` field `name` value `{}` is not a portable mechanism name; the name is the `<role>:<name>` capability vocabulary, so it may not be empty or carry `:`, `/`, `#`, or whitespace ({ONE_MACHINE})",
                self.id, self.name,
            ));
        }
        validate_authorable_handler_kind(&self.handler, &self.id)?;
        validate_handler_shape(&self.handler, &self.id, "[[mechanism]]")?;
        if self.protocol == 0 {
            return Err(format!(
                "[[mechanism]] `{}` field `protocol` value `0` is forbidden; provider protocol versions start at 1 ({ONE_MACHINE})",
                self.id,
            ));
        }
        if let Err(fault) = declarant_path(&self.config_schema) {
            return Err(declarant_path_error(
                "[[mechanism]]",
                &self.id,
                "config_schema",
                &self.config_schema,
                fault,
                ONE_MACHINE,
            ));
        }
        Ok(())
    }
}

/// The authorable handler kinds for a provider.
///
/// A mechanism speaks a numbered provider `protocol` and answers a freshness
/// probe, so its handler has to be an *implementation* a host can run and
/// version: `script`, `binary`, or `native`.
///
/// `builtin` is refused because builtin mechanisms are engine-synthetic
/// descriptors — the engine mints them for what it already implements
/// (`org.vibevm/vibe#cargo` and its siblings), and a manifest that could
/// author one would be naming an engine internal by string, exactly what the
/// `[[extension]]` reserved-builtin guard exists to prevent. `agent` is
/// refused because a prompt cannot honour a numbered protocol or a
/// deterministic freshness probe; an agent belongs at an extension point,
/// not in the build/package/deploy plane.
fn validate_authorable_handler_kind(handler: &ExtensionHandler, id: &str) -> Result<(), String> {
    match handler {
        ExtensionHandler::Builtin { name } => Err(format!(
            "[[mechanism]] `{id}` handler kind `builtin` (name `{name}`) is not authorable; builtin mechanisms are engine-synthetic descriptors the engine mints for its own providers, never manifest-authored — declare `script`, `binary`, or `native` ({ONE_MACHINE})",
        )),
        ExtensionHandler::Agent { .. } => Err(format!(
            "[[mechanism]] `{id}` handler kind `agent` is not authorable; a prompt cannot honour a numbered provider `protocol` or a deterministic freshness probe — declare `script`, `binary`, or `native`, or use `[[extension]]` for an agent contribution ({ONE_MACHINE})",
        )),
        ExtensionHandler::Script { .. }
        | ExtensionHandler::Binary { .. }
        | ExtensionHandler::Native { .. } => Ok(()),
    }
}

/// Who owns a mechanism target's freshness probe. Exactly `engine` or
/// `provider` — there is no third value and no default.
///
/// ```
/// use vibe_core::manifest::MechanismFreshness;
///
/// assert_eq!("engine".parse(), Ok(MechanismFreshness::Engine));
/// assert_eq!("provider".parse(), Ok(MechanismFreshness::Provider));
/// assert!("sometimes".parse::<MechanismFreshness>().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechanismFreshness {
    Engine,
    Provider,
}

impl MechanismFreshness {
    /// The exact lowercase wire spelling.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Provider => "provider",
        }
    }
}

impl fmt::Display for MechanismFreshness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MechanismFreshness {
    type Err = MechanismFreshnessParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "engine" => Ok(Self::Engine),
            "provider" => Ok(Self::Provider),
            _ => Err(MechanismFreshnessParseError {
                input: input.to_owned(),
            }),
        }
    }
}

/// A value that is neither `engine` nor `provider`.
///
/// ```
/// use vibe_core::manifest::MechanismFreshness;
///
/// let error = "sometimes".parse::<MechanismFreshness>().unwrap_err();
/// assert_eq!(error.input(), "sometimes");
/// assert!(error.to_string().contains("engine, provider"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanismFreshnessParseError {
    input: String,
}

impl fmt::Display for MechanismFreshnessParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid mechanism freshness `{}`; expected one of: engine, provider ({ONE_MACHINE})",
            self.input,
        )
    }
}

impl std::error::Error for MechanismFreshnessParseError {}

impl MechanismFreshnessParseError {
    /// Return the exact value that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

/// The host-owned `[mechanisms]` route table — logical key to exact provider
/// identity. Routes are host controls; selection execution lands later.
///
/// A route table is a **map**: exactly one entry answers a key, and no entry
/// shadows another, so the order entries were written in carries no meaning.
/// It is not preserved either — a rewrite renders map keys sorted — and
/// equality ignores it. Iteration order is insertion order, a deterministic
/// convenience for a programmatically built table and nothing more.
///
/// ```
/// use vibe_core::manifest::{MechanismRoutes, ProviderPin};
///
/// let pin: ProviderPin = "org.example/build-tools#cargo-v2".parse().unwrap();
/// let mut routes = MechanismRoutes::default();
/// routes.insert("build:cargo".parse().unwrap(), pin.clone());
/// assert_eq!(routes.len(), 1);
/// assert_eq!(routes.get("build:cargo"), Some(&pin));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MechanismRoutes(IndexMap<MechanismKey, ProviderPin>);

impl MechanismRoutes {
    /// The route for one logical key, if authored.
    pub fn get(&self, key: &str) -> Option<&ProviderPin> {
        key.parse::<MechanismKey>()
            .ok()
            .and_then(|key| self.0.get(&key))
    }

    /// The number of authored routes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the table carries no routes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate the routes in insertion order. Deterministic, but not a
    /// precedence order and not the render order.
    pub fn iter(&self) -> impl Iterator<Item = (&MechanismKey, &ProviderPin)> {
        self.0.iter()
    }

    /// Insert one route, replacing any earlier entry for the same key and
    /// keeping the original iteration position. Both halves are
    /// already-validated types, so a route table can only ever hold a legal
    /// key and an exact provider identity.
    pub fn insert(&mut self, key: MechanismKey, pin: ProviderPin) {
        self.0.insert(key, pin);
    }
}

/// Validate mechanism declarations: shape, uniqueness, and the
/// project-or-package role law. A pure virtual `[workspace]` may route and
/// select but may not declare a provider — the `[[extension]]` precedent.
pub(crate) fn validate_mechanism_declarations(
    mechanisms: &[MechanismDecl],
    has_project: bool,
    has_package: bool,
) -> Result<(), String> {
    if !mechanisms.is_empty() && !has_project && !has_package {
        return Err(format!(
            "[[mechanism]] is legal only in a manifest with `[project]` or `[package]`; a pure virtual `[workspace]` may route `[mechanisms]` but may not declare a provider ({ONE_MACHINE})",
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for mechanism in mechanisms {
        mechanism.validate()?;
        if !ids.insert(mechanism.id.as_str()) {
            return Err(format!(
                "duplicate [[mechanism]] field `id` value `{}`; provider ids are unique within the declaring manifest ({ONE_MACHINE})",
                mechanism.id,
            ));
        }
    }
    Ok(())
}
