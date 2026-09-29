//! Producer-owned opaque references, never new content hashes or authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#IDENTITY-OWNER-REUSE");
use super::{EnvironmentError, invalid};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference(String);
impl Reference {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#INJECTED-CONSTRUCTION-AUTHORITY",
        r = 1
    )]
    pub fn new(value: impl Into<String>) -> Result<Self, EnvironmentError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(invalid("reference", "reference must be nonblank"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
macro_rules! role_ids {
    ($($name:ident),+ $(,)?) => {$ (
        /// Opaque role reference and its producer/admitting caller. Equal text
        /// in another role has no conversion or physical-authority implication.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name { reference: Reference, producer: Reference }
        impl $name {
            #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-ROLE-SEPARATION", r = 1)]
            pub fn new(reference: impl Into<String>, producer: impl Into<String>) -> Result<Self, EnvironmentError> {
                Ok(Self { reference: Reference::new(reference)?, producer: Reference::new(producer)? })
            }
            pub fn reference(&self) -> &Reference { &self.reference }
            pub fn producer(&self) -> &Reference { &self.producer }
        }
    )+};
}
role_ids!(
    DefinitionId,
    LockId,
    ExecutionId,
    TargetInstanceId,
    GenerationId
);
