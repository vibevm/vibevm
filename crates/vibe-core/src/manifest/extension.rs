//! Unified extension declaration and consumer-control manifest grammar.
//!
//! This cell owns what a package or project can declare and how a consuming
//! host can activate or disable it. World-aware collection, ordering, state,
//! and handler execution belong to later lifecycle cells; no row here implies
//! that anything has run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR");

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use specmark::spec;

use crate::lifecycle::{CompilePoint, ExtensionPoint};
use crate::manifest::declarant_path::{declarant_path, declarant_path_error};

mod control;
mod wire;
pub use control::{ExtensionKey, ExtensionUse, ExtensionsControl};
pub(crate) use wire::{ExtensionDeclWire, ExtensionHandlerWire, ExtensionsControlWire};

const CONTRIB_GRAMMAR: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR";
const HANDLER_TABLES: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES";
const HANDLER_KINDS: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#HANDLER-KINDS";
const SELECTOR: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR";
const INTERNALS_FLAG: &str =
    "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILER-INTERNALS-FLAG";
const RESERVED_EXTENSION_ID_PREFIX: &str = "@vibe/";
const RESERVED_INTERNAL_BUILTIN: &str = "package-skill-project";

/// One handler bound to one extension point by a manifest declaration.
///
/// ```
/// use vibe_core::manifest::ExtensionHandler;
///
/// let handler = ExtensionHandler::Builtin { name: "log".into() };
/// assert_eq!(handler.kind(), "builtin");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES")]
pub enum ExtensionHandler {
    Builtin {
        name: String,
    },
    Script {
        base: PathBuf,
    },
    Binary {
        name: String,
    },
    Native {
        crate_dir: Option<PathBuf>,
        prebuilt: Option<BTreeMap<String, PathBuf>>,
    },
    Agent {
        prompt: String,
    },
}

impl ExtensionHandler {
    /// The exact lowercase `handler.kind` spelling.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Builtin { .. } => "builtin",
            Self::Script { .. } => "script",
            Self::Binary { .. } => "binary",
            Self::Native { .. } => "native",
            Self::Agent { .. } => "agent",
        }
    }
}

/// Positive package/path selectors evaluated later by the extension engine.
///
/// ```
/// use vibe_core::manifest::ExtensionAppliesTo;
///
/// let selector = ExtensionAppliesTo {
///     packages: Some(vec!["org.demo/*".into()]),
///     paths: None,
/// };
/// assert_eq!(selector.packages.unwrap(), ["org.demo/*"]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
pub struct ExtensionAppliesTo {
    pub packages: Option<Vec<String>>,
    pub paths: Option<Vec<String>>,
}

/// The four pass-tier declaration kinds.
///
/// ```
/// use vibe_core::manifest::ExtensionPassKind;
///
/// assert_ne!(ExtensionPassKind::Transform, ExtensionPassKind::Backend);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW")]
pub enum ExtensionPassKind {
    Transform,
    Lowering,
    Frontend,
    Backend,
}

/// A named IR level used only by a pass declaration.
///
/// ```
/// use vibe_core::manifest::ExtensionIrLevel;
///
/// assert_ne!(ExtensionIrLevel::Source, ExtensionIrLevel::Emitted);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
pub enum ExtensionIrLevel {
    Source,
    Document,
    Closure,
    Lane,
    Emitted,
}

/// The declaration attached to a `compile:pass` contribution.
///
/// Field combinations deliberately remain unjudged here: R6 owns placement
/// semantics. R2.1 preserves every field and its presence without inventing a
/// conflict rule ahead of that work.
///
/// ```
/// use vibe_core::manifest::{ExtensionIrLevel, ExtensionPass, ExtensionPassKind};
///
/// let pass = ExtensionPass {
///     kind: ExtensionPassKind::Transform,
///     level: Some(ExtensionIrLevel::Closure),
///     from: None, to: None,
///     after: Some("qualify".into()), before: None, replace: None,
///     formats: None, artifact: None,
/// };
/// assert_eq!(pass.level, Some(ExtensionIrLevel::Closure));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW")]
pub struct ExtensionPass {
    pub kind: ExtensionPassKind,
    pub level: Option<ExtensionIrLevel>,
    pub from: Option<ExtensionIrLevel>,
    pub to: Option<ExtensionIrLevel>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub replace: Option<String>,
    pub formats: Option<Vec<String>>,
    pub artifact: Option<String>,
}

/// Arbitrary handler configuration, retained as semantic TOML values.
///
/// ```
/// use vibe_core::manifest::ExtensionConfig;
///
/// let table = toml::from_str("message = 'hello'").unwrap();
/// let config = ExtensionConfig::from_table(table);
/// assert_eq!(config.as_table()["message"].as_str(), Some("hello"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-FIELDS")]
pub struct ExtensionConfig(EqTomlTable);

impl ExtensionConfig {
    pub fn from_table(table: toml::Table) -> Self {
        Self(EqTomlTable(table))
    }

    pub fn as_table(&self) -> &toml::Table {
        &self.0.0
    }

    pub fn into_table(self) -> toml::Table {
        self.0.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.0.is_empty()
    }
}

/// Opaque future applicability guard, preserved without interpretation.
///
/// ```
/// use vibe_core::manifest::ExtensionWhen;
///
/// let guard = ExtensionWhen::from_table(toml::from_str("future = true").unwrap());
/// assert_eq!(guard.as_table()["future"].as_bool(), Some(true));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-FIELDS")]
pub struct ExtensionWhen(EqTomlTable);

impl ExtensionWhen {
    pub fn from_table(table: toml::Table) -> Self {
        Self(EqTomlTable(table))
    }

    pub fn as_table(&self) -> &toml::Table {
        &self.0.0
    }

    pub fn into_table(self) -> toml::Table {
        self.0.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.0.is_empty()
    }
}

/// One declaration in the ordered top-level `[[extension]]` array.
///
/// ```
/// use vibe_core::lifecycle::ExtensionPoint;
/// use vibe_core::manifest::{ExtensionDecl, ExtensionHandler};
///
/// let declaration = ExtensionDecl {
///     id: "announce".into(),
///     point: "phase:build".parse::<ExtensionPoint>().unwrap(),
///     handler: ExtensionHandler::Builtin { name: "log".into() },
///     config: None, auto: None, inputs: None, applies_to: None,
///     compiler_internals: None, pass: None, when: None,
/// };
/// assert!(declaration.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR")]
pub struct ExtensionDecl {
    pub id: String,
    pub point: ExtensionPoint,
    pub handler: ExtensionHandler,
    pub config: Option<ExtensionConfig>,
    pub auto: Option<bool>,
    pub inputs: Option<Vec<String>>,
    pub applies_to: Option<ExtensionAppliesTo>,
    pub compiler_internals: Option<bool>,
    pub pass: Option<ExtensionPass>,
    pub when: Option<ExtensionWhen>,
}

impl ExtensionDecl {
    /// Revalidate a declaration constructed through Rust rather than TOML.
    pub fn validate(&self) -> Result<(), String> {
        validate_handler_shape(&self.handler, &self.id, "[[extension]]")?;

        let is_compile = matches!(self.point, ExtensionPoint::Compile(_));
        let is_phase = matches!(self.point, ExtensionPoint::Phase(_));
        if is_compile
            && !matches!(
                self.handler,
                ExtensionHandler::Builtin { .. } | ExtensionHandler::Native { .. }
            )
        {
            return Err(format!(
                "[[extension]] `{}` point `{}` rejects handler kind `{}`; compile points accept `builtin` or `native` only ({HANDLER_KINDS})",
                self.id,
                self.point,
                self.handler.kind(),
            ));
        }
        if self.auto.is_some() && !is_compile {
            return Err(format!(
                "[[extension]] `{}` field `auto` is present at point `{}`; `auto` is legal only for `compile:*` ({CONTRIB_GRAMMAR})",
                self.id, self.point,
            ));
        }
        if self.inputs.is_some() && !is_phase {
            return Err(format!(
                "[[extension]] `{}` field `inputs` is present at point `{}`; `inputs` is legal only for `phase:*` ({CONTRIB_GRAMMAR})",
                self.id, self.point,
            ));
        }
        if self.applies_to.is_some()
            && !matches!(
                self.point,
                ExtensionPoint::Compile(CompilePoint::Source | CompilePoint::Document)
                    | ExtensionPoint::Slot(_)
            )
        {
            return Err(format!(
                "[[extension]] `{}` field `applies_to` is present at point `{}`; selectors are legal only for `compile:source`, `compile:document`, or `slot:*` ({SELECTOR})",
                self.id, self.point,
            ));
        }

        let is_pass = self.point == ExtensionPoint::Compile(CompilePoint::Pass);
        if is_pass && self.compiler_internals != Some(true) {
            return Err(format!(
                "[[extension]] `{}` point `compile:pass` requires field `compiler_internals = true` ({INTERNALS_FLAG})",
                self.id,
            ));
        }
        if !is_pass && let Some(value) = self.compiler_internals {
            return Err(format!(
                "[[extension]] `{}` field `compiler_internals = {value}` is forbidden at point `{}`; it is reserved for `compile:pass` ({INTERNALS_FLAG})",
                self.id, self.point,
            ));
        }
        if !is_pass && self.pass.is_some() {
            return Err(format!(
                "[[extension]] `{}` field `pass` is forbidden at point `{}`; it is legal only for `compile:pass` ({INTERNALS_FLAG})",
                self.id, self.point,
            ));
        }
        Ok(())
    }
}

/// Validate one handler declaration's structural shape — the shared
/// `REF-HANDLER-TABLES` field laws, used by both `[[extension]]` and
/// `[[mechanism]]` rows. `table` names the declaring table for diagnostics.
///
/// Every path here is judged by the one declarant-path law
/// ([`declarant_path`]), the same law `[[skill]]`, `[[mechanism]]
/// config_schema` and `[[artifacts.*]] inputs` answer to.
pub(crate) fn validate_handler_shape(
    handler: &ExtensionHandler,
    id: &str,
    table: &str,
) -> Result<(), String> {
    match handler {
        ExtensionHandler::Script { base } => {
            if let Err(fault) = declarant_path(base) {
                return Err(declarant_path_error(
                    table,
                    id,
                    "handler.base",
                    base,
                    fault,
                    HANDLER_TABLES,
                ));
            }
            if base.extension().is_some() {
                return Err(format!(
                    "{table} `{id}` field `handler.base` value `{}` must omit its script extension ({HANDLER_TABLES})",
                    base.display(),
                ));
            }
        }
        ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } => {
            if crate_dir.is_none() && prebuilt.is_none() {
                return Err(format!(
                    "{table} `{id}` native handler requires field `crate_dir` or field `prebuilt` ({HANDLER_TABLES})",
                ));
            }
            if let Some(path) = crate_dir
                && let Err(fault) = declarant_path(path)
            {
                return Err(declarant_path_error(
                    table,
                    id,
                    "handler.crate_dir",
                    path,
                    fault,
                    HANDLER_TABLES,
                ));
            }
            if let Some(paths) = prebuilt {
                for (platform, path) in paths {
                    if let Err(fault) = declarant_path(path) {
                        return Err(declarant_path_error(
                            table,
                            id,
                            &format!("handler.prebuilt.{platform}"),
                            path,
                            fault,
                            HANDLER_TABLES,
                        ));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Validate declaration order/identity and the project-or-package role law.
pub(crate) fn validate_extension_declarations(
    extensions: &[ExtensionDecl],
    has_project: bool,
    has_package: bool,
) -> Result<(), String> {
    if !extensions.is_empty() && !has_project && !has_package {
        return Err(format!(
            "[[extension]] is legal only in a manifest with `[project]` or `[package]`; a pure virtual `[workspace]` cannot provide a declaration ({CONTRIB_GRAMMAR})",
        ));
    }

    let mut ids = BTreeSet::new();
    for extension in extensions {
        if extension.id.starts_with(RESERVED_EXTENSION_ID_PREFIX) {
            return Err(format!(
                "[[extension]] field `id` value `{}` uses the reserved `{RESERVED_EXTENSION_ID_PREFIX}` prefix; authored extension ids must not impersonate generated vibe contributions ({CONTRIB_GRAMMAR})",
                extension.id,
            ));
        }
        if matches!(
            &extension.handler,
            ExtensionHandler::Builtin { name } if name == RESERVED_INTERNAL_BUILTIN
        ) {
            return Err(format!(
                "[[extension]] `{}` names internal builtin `{RESERVED_INTERNAL_BUILTIN}`; authored builtins are limited to the public closed vocabulary (`log`) ({HANDLER_TABLES})",
                extension.id,
            ));
        }
        extension.validate()?;
        if !ids.insert(extension.id.as_str()) {
            return Err(format!(
                "duplicate [[extension]] field `id` value `{}`; ids are unique within the declaring manifest ({CONTRIB_GRAMMAR})",
                extension.id,
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct EqTomlTable(toml::Table);

impl PartialEq for EqTomlTable {
    fn eq(&self, other: &Self) -> bool {
        eq_toml_table(&self.0, &other.0)
    }
}

impl Eq for EqTomlTable {}

fn eq_toml_table(left: &toml::Table, right: &toml::Table) -> bool {
    left.len() == right.len()
        && left.iter().all(|(key, value)| {
            right
                .get(key)
                .is_some_and(|other| eq_toml_value(value, other))
        })
}

fn eq_toml_value(left: &toml::Value, right: &toml::Value) -> bool {
    match (left, right) {
        (toml::Value::String(a), toml::Value::String(b)) => a == b,
        (toml::Value::Integer(a), toml::Value::Integer(b)) => a == b,
        (toml::Value::Float(a), toml::Value::Float(b)) => float_key(*a) == float_key(*b),
        (toml::Value::Boolean(a), toml::Value::Boolean(b)) => a == b,
        (toml::Value::Datetime(a), toml::Value::Datetime(b)) => a == b,
        (toml::Value::Array(a), toml::Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| eq_toml_value(x, y))
        }
        (toml::Value::Table(a), toml::Value::Table(b)) => eq_toml_table(a, b),
        _ => false,
    }
}

fn float_key(value: f64) -> u64 {
    if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

#[cfg(test)]
#[path = "extension/tests.rs"]
mod tests;
