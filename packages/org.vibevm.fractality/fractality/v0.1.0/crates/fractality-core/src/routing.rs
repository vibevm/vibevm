//! The routing policy (D-C3-10): tabular data that turns a worker's
//! *capability class* — never its model name — into the caps and grants
//! the need-gate reads. Pool churn is the design condition (FD-16): the
//! swarm's models rotate, so policy rows address classes (weak / medium /
//! strong), and a profile declares which class it is elsewhere. The table
//! stays data in v1 (RD-20 defers a learned router); its features come
//! from the journal's outcome table (D-C3-8).
//!
//! The authored table lives in the `delegation-rules` package
//! (`spec/flows/delegation-rules/routing-policy.toml`, §10.6); [`Default`]
//! here is the same policy compiled in, so the fabric routes correctly
//! with no file, and the file overrides it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

specmark::scope!("spec://fractality/PROP-001#model");

/// A worker's capability class (D-C3-10 / RD-2). Not a model name: the
/// pool rotates, so policy addresses the class a profile declares itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityClass {
    /// Cannot reliably drive recursion (minRLM's nano inversion): route
    /// its work, never make it a spawning root.
    Weak,
    /// Can decompose one level; benefits from an advisor (RD-10 bar).
    Medium,
    /// A strong-coder root: the only class allowed the experimental
    /// depth-2 flag for provably super-linear tasks (RD-2).
    Strong,
}

impl CapabilityClass {
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityClass::Weak => "weak",
            CapabilityClass::Medium => "medium",
            CapabilityClass::Strong => "strong",
        }
    }
}

/// The policy row for one capability class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassPolicy {
    /// Depth cap for a root of this class (feeds `GateInputs.max_depth`).
    /// `0` = no spawning (route/fold only) — the weak-class default.
    pub max_depth: u32,
    /// Whether this class may set the experimental depth-2 flag for
    /// provably super-linear tasks (RD-2: strong-coder roots only).
    pub allow_experimental_depth2: bool,
    /// Whether a caller of this class may consult an advisor (RD-10:
    /// `advisor_enabled ⇐ caller_class ≥ medium`). Advisor itself is
    /// Stage C (PP-003); this row keeps the bar ready.
    pub advisor_enabled: bool,
}

/// The routing policy: a row per capability class. Missing rows fall back
/// to [`Default`], so a partial file never leaves a class unpoliced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingPolicy {
    /// File schema; this build reads and writes `1`.
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub class: BTreeMap<String, ClassPolicy>,
}

fn default_schema() -> u32 {
    1
}

impl Default for RoutingPolicy {
    /// The compiled-in v1 policy — the same table authored in
    /// delegation-rules, a row per class.
    fn default() -> Self {
        let class = [
            CapabilityClass::Weak,
            CapabilityClass::Medium,
            CapabilityClass::Strong,
        ]
        .into_iter()
        .map(|c| (c.as_str().to_owned(), Self::compiled_default_for(c)))
        .collect();
        Self { schema: 1, class }
    }
}

impl RoutingPolicy {
    /// Parses and validates the authored TOML table.
    pub fn from_toml_str(text: &str) -> Result<Self, CoreError> {
        let policy: RoutingPolicy = toml::from_str(text)?;
        if policy.schema != 1 {
            return Err(CoreError::PacketSchema {
                found: policy.schema,
            });
        }
        Ok(policy)
    }

    /// The policy row for a class, falling back to the compiled-in default
    /// when the loaded table omits it — a class is never unpoliced.
    pub fn for_class(&self, class: CapabilityClass) -> ClassPolicy {
        self.class
            .get(class.as_str())
            .copied()
            .unwrap_or_else(|| Self::compiled_default_for(class))
    }

    /// The compiled-in row for a class — the source of truth the authored
    /// `routing-policy.toml` mirrors, and the fallback when a loaded table
    /// omits a class. Total: no lookup, no panic.
    fn compiled_default_for(class: CapabilityClass) -> ClassPolicy {
        match class {
            // Weak: route only, never a spawning root, no advisor.
            CapabilityClass::Weak => ClassPolicy {
                max_depth: 0,
                allow_experimental_depth2: false,
                advisor_enabled: false,
            },
            // Medium: one level of decomposition; clears the advisor bar.
            CapabilityClass::Medium => ClassPolicy {
                max_depth: 1,
                allow_experimental_depth2: false,
                advisor_enabled: true,
            },
            // Strong: one level by default; the only class allowed the
            // experimental depth-2 flag (RD-2).
            CapabilityClass::Strong => ClassPolicy {
                max_depth: 1,
                allow_experimental_depth2: true,
                advisor_enabled: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_encodes_the_class_ladder() {
        let p = RoutingPolicy::default();
        // Weak: route only, no spawn, no advisor.
        let weak = p.for_class(CapabilityClass::Weak);
        assert_eq!(weak.max_depth, 0);
        assert!(!weak.advisor_enabled);
        // Medium: one level of decomposition, advisor bar cleared.
        let medium = p.for_class(CapabilityClass::Medium);
        assert_eq!(medium.max_depth, 1);
        assert!(medium.advisor_enabled);
        // Strong: the only class allowed the experimental depth-2 flag.
        let strong = p.for_class(CapabilityClass::Strong);
        assert!(strong.allow_experimental_depth2);
        assert!(!medium.allow_experimental_depth2);
    }

    #[test]
    fn capability_classes_are_ordered_weak_lt_medium_lt_strong() {
        // The advisor bar (RD-10) is a `>= medium` comparison, so the
        // ordering is load-bearing, not cosmetic.
        assert!(CapabilityClass::Weak < CapabilityClass::Medium);
        assert!(CapabilityClass::Medium < CapabilityClass::Strong);
    }

    #[test]
    fn a_partial_file_falls_back_to_default_for_missing_classes() {
        // Only `strong` authored; weak/medium fall back to compiled-in.
        let text = "\
            schema = 1\n\
            [class.strong]\n\
            max_depth = 2\n\
            allow_experimental_depth2 = true\n\
            advisor_enabled = true\n";
        let p = RoutingPolicy::from_toml_str(text).expect("parses");
        assert_eq!(p.for_class(CapabilityClass::Strong).max_depth, 2);
        // Missing rows still policed by the default.
        assert_eq!(p.for_class(CapabilityClass::Weak).max_depth, 0);
        assert_eq!(p.for_class(CapabilityClass::Medium).max_depth, 1);
    }

    #[test]
    fn foreign_schema_is_refused() {
        let text = "schema = 2\n";
        assert!(RoutingPolicy::from_toml_str(text).is_err());
    }

    #[test]
    fn class_names_serialize_snake_case() {
        let json = serde_json::to_string(&CapabilityClass::Strong).expect("serializes");
        assert_eq!(json, "\"strong\"");
    }
}
