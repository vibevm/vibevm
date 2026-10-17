//! Persistent authored intent, independent of resolved/locked choices.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerConfiguration {
    pub dependency: String,
    pub domain: String,
    pub definition: String,
    pub definition_owner: String,
    pub target: String,
    pub target_owner: String,
    pub instance: String,
    pub configuration: String,
    pub selections: BTreeMap<String, ConsumerSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerSelection {
    pub intent: ConsumerSelectionIntent,
    /// None is omitted; Some([]) is an explicitly empty selection.
    pub members: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerSelectionIntent {
    Exact,
    Additive,
}
