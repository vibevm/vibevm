//! Operation-owned source policy; deferred is never an empty policy.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use crate::{HttpAdmission, RegistryError, source_candidates::SourceChoiceMode};
use std::sync::Arc;
use vibe_core::manifest::EffectiveSourcePolicy;
/// The one operation-owned policy every registry source consults.
///
/// ```
/// use std::sync::Arc;
/// use vibe_core::manifest::EffectiveSourcePolicy;
/// use vibe_registry::source_candidates::SourceChoiceMode;
/// use vibe_registry::source_policy_provider::SourcePolicyProvider;
/// use vibe_registry::{HttpAdmission, RegistryError};
///
/// struct Neutral;
/// impl SourcePolicyProvider for Neutral {
///     fn source_choice_mode(&self) -> SourceChoiceMode {
///         SourceChoiceMode::LegacyKnown
///     }
///     fn policy(&self) -> Result<Arc<EffectiveSourcePolicy>, RegistryError> {
///         Ok(Arc::new(EffectiveSourcePolicy::default()))
///     }
///     fn admission(&self) -> Result<Arc<HttpAdmission>, RegistryError> {
///         Ok(Arc::new(HttpAdmission::new(Vec::<String>::new())?))
///     }
/// }
///
/// let provider = Neutral;
/// assert!(!provider.policy().unwrap().has_preference());
/// assert!(provider.admission().unwrap().check_address("https://example.invalid/x").is_ok());
/// ```
pub trait SourcePolicyProvider: Send + Sync {
    fn source_choice_mode(&self) -> SourceChoiceMode;
    fn policy(&self) -> Result<Arc<EffectiveSourcePolicy>, RegistryError>;
    fn admission(&self) -> Result<Arc<HttpAdmission>, RegistryError>;
}
