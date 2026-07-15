//! Reusable enforcement gates over a [`Registry`] (PROP-039 §8.4, §12.2).
//!
//! A consumer wires these into its own CI / `self-check`. [`legibility`]
//! enforces the human-legibility discipline (§8.4) against the **English**
//! surface — every registered action's presentation `name` and `description`
//! (the inline `default_en`, which never misses) must be present, non-empty,
//! and non-placeholder. [`reachable`] is the enumerable-registry golden
//! (§12.2): every registered action round-trips through [`Registry::get`] and
//! its [`ActionAddr`] re-parses to the same identity.
//!
//! Both enumerate the registry (§4.3) and return a typed list of violations
//! rather than panicking, so the caller decides how to report. They are pure,
//! allocation-light checks with zero rendering dependencies (§1
//! `#no-render-dep`).
//!
//! Spec: [PROP-039 §8.4](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#legibility-gate),
//! [§12.2](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#gates).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#gates");

use std::fmt;

use crate::address::ActionAddr;
use crate::registry::Registry;

/// The leading placeholder tokens a presentation string must not lead with,
/// matched case-insensitively against the leading alphanumeric run (§8.4).
const PLACEHOLDER_TOKENS: [&str; 3] = ["todo", "fixme", "xxx"];

/// A human-legibility violation (PROP-039 §8.4): a presentation `field`
/// (`name` or `description`) of the action at `address` is empty,
/// whitespace-only, or a leading placeholder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LegibilityViolation {
    /// The offending action's address, in textual form.
    pub address: String,
    /// Which presentation field failed — `"name"` or `"description"`.
    pub field: &'static str,
    /// Why it failed, in human-legible English.
    pub reason: String,
}

impl fmt::Display for LegibilityViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} is {}", self.address, self.field, self.reason)
    }
}

/// A reachability violation (PROP-039 §12.2): the action at `address` is not
/// reachable by enumeration + [`Registry::get`], or its address does not
/// round-trip through parsing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ReachabilityViolation {
    /// The offending action's address, in textual form.
    pub address: String,
    /// Why it failed, in human-legible English.
    pub reason: String,
}

impl fmt::Display for ReachabilityViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.address, self.reason)
    }
}

/// Classify a presentation string, returning *why* it is illegible — or `None`
/// when it is acceptable. Rejects empty / whitespace-only text and a leading
/// `TODO` / `FIXME` / `xxx` placeholder token (case-insensitive).
fn illegible_reason(text: &str) -> Option<&'static str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some("empty or whitespace-only");
    }
    let lead = trimmed
        .chars()
        .take_while(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if PLACEHOLDER_TOKENS.contains(&lead.as_str()) {
        return Some("a leading TODO/FIXME/xxx placeholder");
    }
    None
}

/// Enforce the human-legibility discipline over every registered action
/// (PROP-039 §8.4). Enumerates the registry (§4.3) and asserts each action's
/// **English** presentation `name` and `description` (the inline `default_en`)
/// are present, non-empty, and non-placeholder. Returns every violation — each
/// naming the offending address and field — or `Ok(())` when the registry is
/// clean.
pub fn legibility(reg: &Registry) -> Result<(), Vec<LegibilityViolation>> {
    let mut violations = Vec::new();
    for action in reg.iter() {
        let address = action.addr().to_string();
        let presentation = action.presentation();
        if let Some(reason) = illegible_reason(presentation.name().default_en()) {
            violations.push(LegibilityViolation {
                address: address.clone(),
                field: "name",
                reason: reason.to_owned(),
            });
        }
        if let Some(reason) = illegible_reason(presentation.description().default_en()) {
            violations.push(LegibilityViolation {
                address,
                field: "description",
                reason: reason.to_owned(),
            });
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// The enumerable-registry golden (PROP-039 §12.2). Asserts every registered
/// action is reachable by [`Registry::get`] — the lookup round-trips to the
/// same action — and that its [`ActionAddr`] re-parses to the same identity.
/// Returns every violation, or `Ok(())` when the registry is sound.
pub fn reachable(reg: &Registry) -> Result<(), Vec<ReachabilityViolation>> {
    let mut violations = Vec::new();
    for action in reg.iter() {
        let addr = action.addr();
        let address = addr.to_string();

        match reg.get(addr) {
            Some(found) if found.addr() == addr => {}
            Some(_) => violations.push(ReachabilityViolation {
                address: address.clone(),
                reason: "registry lookup resolved to a different action".to_owned(),
            }),
            None => violations.push(ReachabilityViolation {
                address: address.clone(),
                reason: "not reachable by registry lookup".to_owned(),
            }),
        }

        match ActionAddr::parse(&address) {
            Ok(reparsed) if &reparsed == addr => {}
            Ok(_) => violations.push(ReachabilityViolation {
                address: address.clone(),
                reason: "address does not round-trip through parse".to_owned(),
            }),
            Err(_) => violations.push(ReachabilityViolation {
                address: address.clone(),
                reason: "address failed to re-parse".to_owned(),
            }),
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::invoke::InvokeOutcome;

    fn addr(s: &str) -> ActionAddr {
        ActionAddr::parse(s).unwrap()
    }

    fn action(addr_str: &str, name: &'static str, description: &'static str) -> Action {
        Action::builder(addr(addr_str))
            .name_en(name)
            .description_en(description)
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap()
    }

    #[test]
    fn legibility_passes_a_well_formed_registry() {
        let mut reg = Registry::new();
        reg.register(action(
            "action://vibe.tree/copy.markdown",
            "Copy as Markdown",
            "Copy the selected node as a Markdown link",
        ))
        .unwrap();
        assert!(legibility(&reg).is_ok());
    }

    #[test]
    fn legibility_flags_a_placeholder_description_naming_the_address() {
        // `ActionBuilder::build` already rejects empty/whitespace presentation,
        // so the live catch this gate adds over construction is a non-empty
        // *placeholder* description.
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/sort", "Sort", "TODO"))
            .unwrap();
        let violations = legibility(&reg).unwrap_err();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].address, "action://vibe.tree/sort");
        assert_eq!(violations[0].field, "description");
    }

    #[test]
    fn legibility_flags_a_placeholder_name() {
        let mut reg = Registry::new();
        reg.register(action(
            "action://vibe.tree/x",
            "FIXME: name me",
            "A real, meaningful description",
        ))
        .unwrap();
        let violations = legibility(&reg).unwrap_err();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].field, "name");
    }

    #[test]
    fn reachable_passes_a_well_formed_registry() {
        let mut reg = Registry::new();
        reg.register(action("action://core/quit", "Quit", "Quit the application"))
            .unwrap();
        reg.register(action(
            "action://vibe.tree/copy",
            "Copy",
            "Copy the current selection",
        ))
        .unwrap();
        assert!(reachable(&reg).is_ok());
    }

    #[test]
    fn reachable_round_trips_through_an_alias() {
        // An alias does not break the golden: every *registered* action still
        // resolves to itself and re-parses.
        let mut reg = Registry::new();
        reg.register(action(
            "action://vibe.tree/copy.markdown",
            "Copy as Markdown",
            "Copy the selection as Markdown",
        ))
        .unwrap();
        reg.alias(
            addr("action://vibe.tree/copy.md"),
            addr("action://vibe.tree/copy.markdown"),
        )
        .unwrap();
        assert!(reachable(&reg).is_ok());
    }

    #[test]
    fn illegible_reason_accepts_real_text_and_rejects_placeholders() {
        assert!(illegible_reason("Copy as Markdown").is_none());
        assert!(illegible_reason("Todos list").is_none()); // not a bare placeholder token
        assert_eq!(illegible_reason("   "), Some("empty or whitespace-only"));
        assert!(illegible_reason("TODO: finish this").is_some());
        assert!(illegible_reason("xxx").is_some());
        assert!(illegible_reason("FIXME").is_some());
    }

    #[test]
    fn empty_registry_passes_both_gates() {
        let reg = Registry::new();
        assert!(legibility(&reg).is_ok());
        assert!(reachable(&reg).is_ok());
    }

    #[test]
    fn violations_display_legibly() {
        let leg = LegibilityViolation {
            address: "action://vibe.tree/sort".to_owned(),
            field: "description",
            reason: "a leading TODO/FIXME/xxx placeholder".to_owned(),
        };
        assert_eq!(
            leg.to_string(),
            "action://vibe.tree/sort: description is a leading TODO/FIXME/xxx placeholder"
        );
    }
}
