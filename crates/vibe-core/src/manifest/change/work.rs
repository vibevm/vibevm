//! Closed authored DAG declarations; no evidence or scheduling evaluation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-WORK-DAG");
use super::{ChangeError, ChangeErrorKind, declarant_path, invalid};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkNeedKind {
    Specification,
    Implementation,
    Verification,
    Documentation,
    Decision,
    Research,
    Plan,
    Disposition,
    External,
}
impl WorkNeedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Specification => "specification",
            Self::Implementation => "implementation",
            Self::Verification => "verification",
            Self::Documentation => "documentation",
            Self::Decision => "decision",
            Self::Research => "research",
            Self::Plan => "plan",
            Self::Disposition => "disposition",
            Self::External => "external",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkNeed {
    pub kind: WorkNeedKind,
    pub target: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkItem {
    pub id: String,
    #[serde(default)]
    pub predecessors: Vec<String>,
    #[serde(default)]
    pub needs: Vec<WorkNeed>,
    #[serde(default)]
    pub touches: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: u32,
    #[serde(default)]
    items: Vec<WorkItem>,
}
#[derive(Debug, Clone)]
pub struct WorkGraph {
    items: BTreeMap<String, WorkItem>,
}
fn refusal(field: &str, reason: &'static str) -> ChangeError {
    invalid(ChangeErrorKind::Work, field, reason)
}
fn id_valid(id: &str) -> bool {
    id.strip_prefix("work-").is_some_and(|v| {
        !v.is_empty()
            && v.len() <= 59
            && v.as_bytes()[0].is_ascii_lowercase()
            && v.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}
fn canonical_unique<T: Ord>(values: &mut [T], field: &str) -> Result<(), ChangeError> {
    values.sort();
    if values.windows(2).any(|v| v[0] == v[1]) {
        return Err(refusal(field, "duplicate authored declaration"));
    }
    Ok(())
}
impl WorkGraph {
    /// Merge only explicitly supplied authored work documents, then validate
    /// one graph. No filesystem, implicit roots or completion observations.
    pub fn parse_documents(documents: &[&str]) -> Result<Self, ChangeError> {
        let mut items = BTreeMap::new();
        for text in documents {
            let wire: Wire = toml::from_str(text)
                .map_err(|_| refusal("work", "invalid closed work TOML shape"))?;
            if wire.schema != 1 {
                return Err(invalid(
                    ChangeErrorKind::UnknownEpoch,
                    "work.schema",
                    "unsupported work epoch",
                ));
            }
            for mut item in wire.items {
                if !id_valid(&item.id) {
                    return Err(refusal("work.items.id", "invalid work ID"));
                }
                canonical_unique(&mut item.predecessors, "work.items.predecessors")?;
                item.needs.sort_by(|a, b| {
                    a.kind
                        .as_str()
                        .cmp(b.kind.as_str())
                        .then(a.target.cmp(&b.target))
                });
                if item.needs.windows(2).any(|v| v[0] == v[1]) {
                    return Err(refusal(
                        "work.items.needs",
                        "duplicate authored declaration",
                    ));
                }
                canonical_unique(&mut item.touches, "work.items.touches")?;
                if item.needs.iter().any(|n| {
                    n.target.trim().is_empty()
                        || n.target.len() > 1024
                        || n.target.chars().any(char::is_control)
                }) {
                    return Err(refusal(
                        "work.items.needs",
                        "invalid artifact target declaration",
                    ));
                }
                for path in &item.touches {
                    declarant_path(Path::new(path)).map_err(|_| {
                        refusal("work.items.touches", "nonportable relative touches hint")
                    })?;
                }
                if items.insert(item.id.clone(), item).is_some() {
                    return Err(refusal("work.items.id", "duplicate work ID"));
                }
            }
        }
        let graph = Self { items };
        for item in graph.items.values() {
            if item
                .predecessors
                .iter()
                .any(|id| !graph.items.contains_key(id))
            {
                return Err(refusal("work.items.predecessors", "unknown predecessor ID"));
            }
        }
        // Kahn elimination checks cycles without recursive stack growth and
        // does not expose the elimination order as a scheduling decision.
        let mut degree: BTreeMap<_, _> = graph
            .items
            .iter()
            .map(|(id, item)| (id.as_str(), item.predecessors.len()))
            .collect();
        let mut successors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for item in graph.items.values() {
            for predecessor in &item.predecessors {
                successors.entry(predecessor).or_default().push(&item.id);
            }
        }
        let mut zero: Vec<_> = degree
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut removed = 0;
        while let Some(id) = zero.pop() {
            removed += 1;
            for next in successors.get(id).into_iter().flatten() {
                let n = degree.get_mut(next).unwrap();
                *n -= 1;
                if *n == 0 {
                    zero.push(next);
                }
            }
        }
        if removed != graph.items.len() {
            return Err(invalid(
                ChangeErrorKind::Cycle,
                "work.items.predecessors",
                "structural work cycle",
            ));
        }
        Ok(graph)
    }
    pub fn items(&self) -> impl ExactSizeIterator<Item = &WorkItem> {
        self.items.values()
    }
    pub fn item(&self, id: &str) -> Option<&WorkItem> {
        self.items.get(id)
    }
    /// Exact selected predecessor closure, in canonical ID order.
    pub(crate) fn contribution(&self, roots: &[&str]) -> Result<Vec<&WorkItem>, ChangeError> {
        let mut requested = BTreeSet::new();
        for root in roots {
            if !requested.insert(*root) {
                return Err(refusal("work.selection", "duplicate selected work ID"));
            }
            if !self.items.contains_key(*root) {
                return Err(refusal("work.selection", "unknown selected work ID"));
            }
        }
        let mut selected = BTreeSet::new();
        let mut pending: Vec<_> = requested.into_iter().collect();
        while let Some(id) = pending.pop() {
            if selected.insert(id) {
                pending.extend(self.items[id].predecessors.iter().map(String::as_str));
            }
        }
        Ok(selected.into_iter().map(|id| &self.items[id]).collect())
    }
}
