//! Versioned typed configuration-meaning fingerprint; presentation is excluded.
//! Input is authored definition data only, never operation/secret answer values.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS");

use super::*;
use crate::manifest::{FieldError, ParameterType};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

struct Meaning(Sha256);
impl Meaning {
    fn count(&mut self, value: usize) {
        self.0.update((value as u64).to_be_bytes());
    }
    fn text(&mut self, value: &str) {
        self.count(value.len());
        self.0.update(value.as_bytes());
    }
    fn flag(&mut self, value: bool) {
        self.0.update([u8::from(value)]);
    }
    fn number(&mut self, value: i64) {
        self.0.update(value.to_be_bytes());
    }
    fn optional<T>(&mut self, value: Option<T>, write: impl FnOnce(&mut Self, T)) {
        self.flag(value.is_some());
        if let Some(value) = value {
            write(self, value);
        }
    }
    fn set(&mut self, values: &[String]) {
        let values: BTreeSet<_> = values.iter().collect();
        self.count(values.len());
        for value in values {
            self.text(value);
        }
    }
    fn value(&mut self, value: &ParameterValue) {
        match value {
            ParameterValue::Bool(v) => {
                self.text("bool");
                self.flag(*v)
            }
            ParameterValue::Integer(v) => {
                self.text("integer");
                self.number(*v)
            }
            ParameterValue::Text(v) => {
                self.text("text");
                self.text(v)
            }
        }
    }
    fn guards(&mut self, guards: &[ConfigurationGuard]) {
        let values: BTreeSet<_> = guards
            .iter()
            .map(|g| match g {
                ConfigurationGuard::Group(id) => ("group", id),
                ConfigurationGuard::Member(id) => ("member", id),
            })
            .collect();
        self.count(values.len());
        for (kind, id) in values {
            self.text(kind);
            self.text(id);
        }
    }
    fn selection(&mut self, selection: &ConfigurationSelection) {
        self.text(match selection.intent {
            MergeIntent::Additive => "additive",
            MergeIntent::Exact => "exact",
        });
        self.optional(selection.members.as_ref(), |s, v| s.set(v));
    }
    fn cardinality(&mut self, c: Cardinality) {
        self.text(match c {
            Cardinality::ZeroOrMore => "zero-or-more",
            Cardinality::OneOrMore => "one-or-more",
            Cardinality::ZeroOrOne => "zero-or-one",
            Cardinality::ExactlyOne => "exactly-one",
        });
    }
}
impl ConfigurationDecl {
    /// A deterministic fingerprint of validated semantic definition data.
    /// Labels/order/UI default-profile markers do not change meaning; defaults,
    /// relations, visibility, parameter rules, profile contents and effects do.
    /// This does not authenticate source bytes or capture an answer secret.
    /// ```
    /// use vibe_core::manifest::ConfigurationDecl;
    /// let d=ConfigurationDecl::default();
    /// assert!(d.semantic_fingerprint()?.starts_with("configuration-semantic/1:sha256:"));
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    #[track_caller]
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
    )]
    pub fn semantic_fingerprint(&self) -> Result<String, FieldError> {
        self.compile_choices()?;
        let mut s = Meaning(Sha256::new());
        s.0.update(b"vibevm.configuration.semantic/v1\0");
        let mut features: Vec<_> = self.features.iter().collect();
        features.sort_by_key(|f| &f.id);
        s.count(features.len());
        for f in features {
            s.text(&f.id);
            s.flag(match f.visibility {
                ConfigurationFeatureVisibility::Internal => true,
                ConfigurationFeatureVisibility::Public => false,
            });
            s.flag(f.required);
            s.flag(f.default);
            s.optional(f.parent.as_deref(), Meaning::text);
            s.set(&f.requires);
            s.guards(&f.guards);
            let effects: BTreeSet<_> = f
                .effects
                .iter()
                .map(|e| match e {
                    ConfigurationEffect::Dependency { target } => {
                        ("dependency", target.as_str(), "", "")
                    }
                    ConfigurationEffect::Forward {
                        target,
                        feature,
                        strength,
                    } => (
                        "forward",
                        target.as_str(),
                        feature.as_str(),
                        match strength {
                            ForwardStrength::Strong => "strong",
                            ForwardStrength::Weak => "weak",
                        },
                    ),
                    ConfigurationEffect::Subskill { path } => ("subskill", path.as_str(), "", ""),
                    ConfigurationEffect::ConfigValue { name } => {
                        ("config-value", name.as_str(), "", "")
                    }
                })
                .collect();
            s.count(effects.len());
            for (kind, target, feature, strength) in effects {
                s.text(kind);
                s.text(target);
                s.text(feature);
                s.text(strength);
            }
        }
        let mut groups: Vec<_> = self.groups.iter().collect();
        groups.sort_by_key(|g| match g {
            ConfigurationGroupDecl::Choice { id, .. }
            | ConfigurationGroupDecl::Closure { id, .. } => id,
        });
        s.count(groups.len());
        for g in groups {
            match g {
                ConfigurationGroupDecl::Choice {
                    id,
                    cardinality,
                    members,
                    defaults,
                    guards,
                    ..
                } => {
                    s.text(id);
                    s.text("choice");
                    s.cardinality(*cardinality);
                    s.set(members);
                    s.optional(defaults.as_ref(), |s, v| s.set(v));
                    s.guards(guards)
                }
                ConfigurationGroupDecl::Closure {
                    id,
                    cardinality,
                    members,
                    fallback,
                    ..
                } => {
                    s.text(id);
                    s.text("closure");
                    s.cardinality(*cardinality);
                    s.set(members);
                    s.optional(fallback.as_ref(), |s, v| s.set(v));
                }
            }
        }
        let mut params: Vec<_> = self.parameters.iter().collect();
        params.sort_by_key(|p| &p.id);
        s.count(params.len());
        for p in params {
            s.text(&p.id);
            s.text(match p.kind {
                ParameterType::String => "string",
                ParameterType::Bool => "bool",
                ParameterType::Integer => "integer",
                ParameterType::Choice => "choice",
                ParameterType::Path => "path",
            });
            s.flag(p.required);
            s.flag(p.secret);
            s.flag(p.must_exist);
            s.optional(p.min, Meaning::number);
            s.optional(p.max, Meaning::number);
            s.optional(p.default.as_ref(), Meaning::value);
            s.set(
                &p.choices
                    .iter()
                    .map(|c| c.value.clone())
                    .collect::<Vec<_>>(),
            );
        }
        let mut profiles: Vec<_> = self.profiles.iter().collect();
        profiles.sort_by_key(|p| &p.id);
        s.count(profiles.len());
        for p in profiles {
            s.text(&p.id);
            s.set(&p.features);
            s.count(p.selections.len());
            for (id, answer) in &p.selections {
                s.text(id);
                s.selection(answer);
            }
            s.count(p.parameters.len());
            for (id, value) in &p.parameters {
                s.text(id);
                s.value(value);
            }
        }
        s.count(self.config_values.len());
        for (id, value) in &self.config_values {
            s.text(id);
            s.value(value);
        }
        Ok(format!(
            "configuration-semantic/1:sha256:{:x}",
            s.0.finalize()
        ))
    }
}

#[cfg(test)]
mod tests;
