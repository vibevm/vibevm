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
