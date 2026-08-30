//! The client projections' structured config — §6.3.0.3's one member,
//! validated at `plan`.
//!
//! > "Its strict config requires a nonempty unique `components` subset of
//! > `skills|mcp`; only those explicitly requested portable-v1 components
//! > may be emitted."
//!
//! Strict like every table this project owns, and a SET rather than a list.
//! The distinction is load-bearing: `["mcp", "skills"]` and
//! `["skills", "mcp"]` request the same projection, so they must produce the
//! same bytes and the same fingerprint. Storing the authored array as
//! authored would make a reordering of two words a different artifact, and
//! §4.1's freshness would then invalidate a target nobody changed.
//!
//! The vocabulary is closed because §6.2 closes it: "Portable v1 components
//! are Agent Skills and MCP servers only. Commands, hooks, agents and LSPs
//! are client projections, not invented portable fields." A third word here
//! would be this engine inventing one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use vibe_core::manifest::ExtensionConfig;

use crate::mechanism::MechanismError;
use crate::mechanism::error::preview;

/// The engine-owned members a target may not set, each with the reason.
///
/// The same three the two §6 packaging configs refuse by name, for the same
/// sentence of §3.2: a reader reaching for them is reaching for the
/// engine's own ownership of placement and environment.
const ENGINE_OWNED: [(&str, &str); 3] = [
    (
        "output",
        "the projection's placement is engine-owned (§3.2: a provider cannot mint an output \
         path); the projection IS the target's own package directory",
    ),
    (
        "output_dir",
        "the projection's placement is engine-owned (§3.2: a provider cannot mint an output path)",
    ),
    (
        "env",
        "a provider receives no environment from the manifest; VibeVM never places configuration \
         bytes into a provider environment",
    ),
];

/// The member that carries the requested component set.
const COMPONENTS: &str = "components";

/// One portable v1 component a projection may be asked to emit.
///
/// The declaration ORDER is the canonical order: `skills` before `mcp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PortableComponent {
    Skills,
    Mcp,
}

impl PortableComponent {
    /// The exact authored spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Mcp => "mcp",
        }
    }

    /// The closed vocabulary, in canonical order.
    pub(crate) const ALL: [Self; 2] = [Self::Skills, Self::Mcp];

    /// One authored word as a component, or nothing.
    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }

    /// The vocabulary a refusal lists.
    fn vocabulary() -> String {
        Self::ALL
            .iter()
            .map(|component| component.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The validated `config` table of one client-projection target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientProjectionConfig {
    /// The requested components as a SET, always in canonical order.
    components: Vec<PortableComponent>,
}

impl ClientProjectionConfig {
    /// Read and validate one target's `config` table.
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, MechanismError> {
        let refuse = |member: &str, reason: String| MechanismError::Config {
            target: target.to_owned(),
            member: preview(member),
            reason,
        };
        let mut requested: Option<Vec<PortableComponent>> = None;
        if let Some(config) = config {
            for (member, value) in config.as_table() {
                if let Some((_, reason)) = ENGINE_OWNED.iter().find(|(name, _)| name == member) {
                    return Err(refuse(member, (*reason).to_owned()));
                }
                if member.as_str() != COMPONENTS {
                    return Err(refuse(
                        member,
                        format!(
                            "unknown member; the client-projection config is `{COMPONENTS}` and \
                             nothing else — the CLIENT is the selected provider's identity, never \
                             a config value"
                        ),
                    ));
                }
                requested = Some(components(target, value)?);
            }
        }
        let components = requested.ok_or_else(|| {
            refuse(
                COMPONENTS,
                format!(
                    "required; name the portable components this projection emits, as a nonempty \
                     unique subset of [{}]",
                    PortableComponent::vocabulary()
                ),
            )
        })?;
        Ok(Self { components })
    }

    /// The requested components, in canonical order.
    pub(crate) fn components(&self) -> &[PortableComponent] {
        &self.components
    }

    /// Whether one component was requested.
    pub(crate) fn wants(&self, component: PortableComponent) -> bool {
        self.components.contains(&component)
    }

    /// The canonical rendering a summary, an evidence line and the
    /// fingerprint all use — one spelling of the set, so no two of them can
    /// disagree about what was requested.
    pub(crate) fn rendered(&self) -> String {
        self.components
            .iter()
            .map(|component| component.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// One `components` array: nonempty, unique, a subset of the closed
/// vocabulary, stored in canonical order.
fn components(target: &str, value: &toml::Value) -> Result<Vec<PortableComponent>, MechanismError> {
    let refuse = |member: String, reason: String| MechanismError::Config {
        target: target.to_owned(),
        member: preview(&member),
        reason,
    };
    let array = value.as_array().ok_or_else(|| {
        refuse(
            COMPONENTS.to_owned(),
            format!(
                "expected an array of component names, found {}",
                preview(value.type_str())
            ),
        )
    })?;
    if array.is_empty() {
        return Err(refuse(
            COMPONENTS.to_owned(),
            "an empty array requests nothing; a projection that emits no portable component is \
             not a projection of the plugin, so the target refuses instead of producing an empty \
             directory"
                .to_owned(),
        ));
    }
    let mut selected: Vec<PortableComponent> = Vec::with_capacity(array.len());
    for (index, item) in array.iter().enumerate() {
        let member = format!("{COMPONENTS}[{index}]");
        let spelled = item.as_str().ok_or_else(|| {
            refuse(
                member.clone(),
                format!(
                    "expected a component name, found {}",
                    preview(item.type_str())
                ),
            )
        })?;
        let component = PortableComponent::parse(spelled).ok_or_else(|| {
            refuse(
                member.clone(),
                format!(
                    "`{}` is not a portable v1 component; §6.2 admits [{}] and nothing else — \
                     commands, hooks, agents and LSPs are client projections, not portable fields",
                    preview(spelled),
                    PortableComponent::vocabulary()
                ),
            )
        })?;
        if selected.contains(&component) {
            return Err(refuse(
                member,
                format!(
                    "`{}` is named twice; `{COMPONENTS}` is a SET, and a repeated member is an \
                     authored spelling with no second meaning",
                    component.as_str()
                ),
            ));
        }
        selected.push(component);
    }
    // Canonical order, so two authored spellings of one set produce one
    // projection, one fingerprint and one record.
    selected.sort_unstable();
    Ok(selected)
}
