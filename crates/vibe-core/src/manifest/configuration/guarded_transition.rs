//! Pure exact gate-input transition; source/operation authority stays with caller.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION");
use super::*;
use specmark::spec;
impl PackageConfiguration {
    /// Only one explicit independent gate changes; both old/new histories must
    /// independently reconstruct from the same definition and declarations.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
    pub fn is_guarded_gate_transition(&self, next: &Self) -> bool {
        if self.consumers.len() != 1
            || next.consumers.len() != 1
            || !self.forwarded_inputs.is_empty()
            || !next.forwarded_inputs.is_empty()
            || !self.submitted_inputs.is_empty()
            || !next.submitted_inputs.is_empty()
            || self.declarations != next.declarations
            || self.definition.semantic_fingerprint().ok()
                != next.definition.semantic_fingerprint().ok()
            || self.definition.semantic_fingerprint().is_err()
            || self.definition.groups.len() != 2
        {
            return false;
        }
        let guarded: Vec<_> = self
            .definition
            .groups
            .iter()
            .filter_map(|g| match g {
                ConfigurationGroupDecl::Choice { id, guards, .. } if !guards.is_empty() => {
                    Some((id, guards))
                }
                _ => None,
            })
            .collect();
        let [(child, guards)] = guarded.as_slice() else {
            return false;
        };
        let [ConfigurationGuard::Member(member)] = guards.as_slice() else {
            return false;
        };
        let gate = self.definition.groups.iter().find_map(|g| match g {
            ConfigurationGroupDecl::Choice {
                id,
                members,
                guards,
                cardinality,
                ..
            } if id != *child
                && guards.is_empty()
                && *cardinality == Cardinality::ExactlyOne
                && members.contains(member) =>
            {
                Some(id)
            }
            _ => None,
        });
        let Some(gate) = gate else { return false };
        let Some((consumer, old)) = self.consumers.first_key_value() else {
            return false;
        };
        let Some(new) = next.consumers.get(consumer) else {
            return false;
        };
        if old.policy != crate::configuration::ChoicePolicy::UseDeclaredDefaults
            || old.selections.contains_key(*child)
            || new.selections.contains_key(*child)
            || old.profile.is_some()
            || old.default_features
            || !old.features.is_empty()
            || !old.parameters.is_empty()
        {
            return false;
        }
        let (Some(a), Some(b)) = (old.selections.get(gate), new.selections.get(gate)) else {
            return false;
        };
        if a == b
            || a.intent != MergeIntent::Exact
            || b.intent != MergeIntent::Exact
            || a.members.as_ref().is_none_or(|m| m.len() != 1)
            || b.members.as_ref().is_none_or(|m| m.len() != 1)
        {
            return false;
        }
        // The transition must actually toggle the guard, not exchange two
        // members that both leave the guarded child active (or both dormant).
        let old_on = a
            .members
            .as_ref()
            .is_some_and(|members| members.contains(member));
        let new_on = b
            .members
            .as_ref()
            .is_some_and(|members| members.contains(member));
        if old_on == new_on {
            return false;
        }
        let mut changed = old.clone();
        changed.selections.insert(gate.clone(), b.clone());
        if &changed != new {
            return false;
        }
        for record in [self, next] {
            let expected = Self::evaluate_parts(
                "guarded gate transition".into(),
                crate::PackageKind::Flow,
                &record.definition,
                &record.declarations,
                &record.consumers,
                &[],
            );
            if expected.as_ref().ok() != Some(record) {
                return false;
            }
        }
        true
    }
}
