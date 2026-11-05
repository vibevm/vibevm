//! Checked metadata serialization on the existing normalized requires owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-INTENT");
use super::Requires;
use super::wire::RequiresWire;
use crate::manifest::FieldPath;
use serde::{Serialize, Serializer};
use std::collections::BTreeSet;

impl Requires {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#OPTIONAL-DECLARATIONS"
    )]
    pub(crate) fn validate_configuration_metadata(&self) -> Result<(), String> {
        let mut declared = BTreeSet::new();
        for (group, name) in self.iter_pkgrefs() {
            if let Some(group) = group {
                let key = format!("{group}/{name}");
                if !declared.insert(key.clone()) {
                    return Err(format!("dependency `{key}` declared more than once"));
                }
            }
        }
        for key in self.optional.keys() {
            if !declared.contains(key) {
                return Err(format!(
                    "optional flag for `{key}` has no declared requires edge"
                ));
            }
        }
        for (key, input) in &self.configurations {
            if !declared.contains(key) {
                return Err(format!(
                    "configuration for `{key}` has no declared requires edge"
                ));
            }
            input
                .validate_shape_at(
                    &FieldPath::root()
                        .table("requires")
                        .table("packages")
                        .table(key)
                        .table("configuration"),
                )
                .map_err(|e| format!("{}: {}", e.field, e.message))?;
        }
        Ok(())
    }
}
impl Serialize for Requires {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.validate_configuration_metadata()
            .map_err(serde::ser::Error::custom)?;
        RequiresWire::from(self.clone()).serialize(serializer)
    }
}

#[cfg(test)]
mod tests;
