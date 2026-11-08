//! Desired activation shared by existing projection receipt owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION");

pub use vibe_wire::generated::shared::ActivationIntent;

/// Legacy missing intent retains enabled-by-default behavior and unknown actor.
/// ```
/// assert!(vibe_agent_projection::activation::desired_enabled(None));
/// ```
pub fn desired_enabled(intent: Option<&ActivationIntent>) -> bool {
    intent.is_none_or(|intent| intent.enabled)
}

/// Validate explicit command provenance without consulting an ambient identity.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn validate(intent: &ActivationIntent) -> anyhow::Result<()> {
    for value in std::iter::once(intent.provenance.as_str()).chain(intent.actor.as_deref()) {
        if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            anyhow::bail!(
                "activation provenance must be nonempty, bounded and free of control characters"
            );
        }
    }
    Ok(())
}

/// How an explicit command may claim an unreceipted builtin entry that an
/// older vibe wrote before ownership receipts existed.
///
/// Byte-identical canonical content is always adoptable by an explicit
/// command; `force` additionally lets that command replace *stale* builtin
/// content. Observation (`status`) never adopts.
///
/// ```
/// use vibe_agent_projection::activation::BuiltinAdoption;
/// let adoption = BuiltinAdoption::explicit("vibe mcp upgrade");
/// assert!(!adoption.force);
/// assert_eq!(adoption.provenance(true), "vibe mcp upgrade:explicit-adoption");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub struct BuiltinAdoption {
    /// The explicit command performing the adoption (`vibe mcp install`, …).
    pub command: &'static str,
    /// Replace stale unreceipted builtin content instead of refusing it.
    pub force: bool,
}

impl BuiltinAdoption {
    /// Adopt byte-identical canonical content only.
    pub fn explicit(command: &'static str) -> Self {
        Self {
            command,
            force: false,
        }
    }

    /// Adopt and, when stale, replace unreceipted builtin content.
    pub fn forced(command: &'static str) -> Self {
        Self {
            command,
            force: true,
        }
    }

    /// The durable provenance recorded on the adopted receipt row. Only an
    /// adoption that actually replaced stale bytes is a *forced* adoption;
    /// claiming identical canonical bytes stays an explicit adoption even
    /// when `--force` was passed.
    ///
    /// ```
    /// use vibe_agent_projection::activation::BuiltinAdoption;
    /// let forced = BuiltinAdoption::forced("vibe mcp install");
    /// assert_eq!(forced.provenance(false), "vibe mcp install:explicit-adoption");
    /// assert_eq!(forced.provenance(true), "vibe mcp install:forced-adoption");
    /// ```
    pub fn provenance(&self, replaced: bool) -> String {
        if self.force && replaced {
            format!("{}:forced-adoption", self.command)
        } else {
            format!("{}:explicit-adoption", self.command)
        }
    }

    /// The durable intent recorded on the adopted receipt row.
    pub fn intent(&self, replaced: bool) -> ActivationIntent {
        ActivationIntent {
            enabled: true,
            actor: None,
            provenance: self.provenance(replaced),
        }
    }
}
