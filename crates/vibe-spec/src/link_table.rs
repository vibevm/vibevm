//! Link tables (PROP-035 §10) — the vtable analogue *(provisional)*.
//!
//! Inline mode is a devirtualized/non-virtual call (bound statically); structural
//! mode is a virtual call (late-bound at runtime); a **link table is the vtable**
//! — an index the compiler builds once so the runtime dispatches cheaply instead
//! of searching. Built at install-time by walking every directive edge
//! (`#use` / `#source` / `#embed` + `@spec`) reachable from a seed, it gives the
//! structural executor the "global knowledge" it otherwise lacks — the edges are
//! computed by code, not rebuilt in the agent's context — and makes the
//! hand-drawn `#source` edges verifiable against the real graph.
//!
//! This is the core (the graph + a deterministic dump). A persisted on-disk
//! format (a `specmap.json` sibling) and the structural consumer are follow-ups.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use crate::address::SpecAddress;
use crate::directives::{DirectiveKind, Directives};
use crate::embed::SectionSource;

/// The directive edges of a document closure, keyed by node
/// (`SpecAddress::without_pin`) and sorted for a stable dump.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkTable {
    /// `#use` + `@spec` dependency edges.
    pub uses: BTreeMap<String, Vec<String>>,
    /// `#source` contract→implementation edges.
    pub sources: BTreeMap<String, Vec<String>>,
    /// `#embed` macro edges.
    pub embeds: BTreeMap<String, Vec<String>>,
}

/// Why a link table could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LinkTableError {
    #[error("cannot load {addr}: {reason}")]
    Unresolved { addr: String, reason: String },
}

/// Walk every directive edge reachable from `seed` and record it. Cycle-safe: a
/// node is visited once.
pub fn build_link_table(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<LinkTable, LinkTableError> {
    let mut table = LinkTable::default();
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue = vec![seed.clone()];

    while let Some(addr) = queue.pop() {
        let key = addr.without_pin();
        if !visited.insert(key.clone()) {
            continue;
        }

        let text = source
            .section_text(&addr)
            .map_err(|reason| LinkTableError::Unresolved {
                addr: key.clone(),
                reason,
            })?;
        let directives = Directives::parse(&text);

        for d in &directives.directives {
            let bucket = match d.kind {
                DirectiveKind::Use => &mut table.uses,
                DirectiveKind::Source => &mut table.sources,
                DirectiveKind::Embed => &mut table.embeds,
            };
            bucket
                .entry(key.clone())
                .or_default()
                .push(d.address.without_pin());
            queue.push(d.address.clone());
        }
        for u in &directives.in_place_uses {
            table
                .uses
                .entry(key.clone())
                .or_default()
                .push(u.address.without_pin());
            queue.push(u.address.clone());
        }
    }

    Ok(table)
}

impl LinkTable {
    /// A stable, tab-separated dump — one `kind<TAB>from<TAB>to` edge per line,
    /// sorted (the `BTreeMap` key order). The on-disk index format later.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (kind, map) in [
            ("use", &self.uses),
            ("source", &self.sources),
            ("embed", &self.embeds),
        ] {
            for (from, tos) in map {
                for to in tos {
                    writeln!(out, "{kind}\t{from}\t{to}").unwrap();
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockSource(HashMap<String, String>);

    impl MockSource {
        fn new(pairs: &[(&str, &str)]) -> Self {
            MockSource(
                pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            )
        }
    }

    impl SectionSource for MockSource {
        fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
            self.0
                .get(&addr.without_pin())
                .cloned()
                .ok_or_else(|| "not in mock".to_string())
        }
    }

    fn seed() -> SpecAddress {
        SpecAddress::parse("spec://vibevm/a#r").unwrap()
    }

    #[test]
    fn records_each_edge_kind() {
        let src = MockSource::new(&[
            (
                "spec://vibevm/a#r",
                "#use spec://vibevm/b#r\n#source spec://vibevm/c#r\n#embed spec://vibevm/d#r",
            ),
            ("spec://vibevm/b#r", "leaf"),
            ("spec://vibevm/c#r", "leaf"),
            ("spec://vibevm/d#r", "leaf"),
        ]);
        let table = build_link_table(&seed(), &src).unwrap();
        assert_eq!(table.uses["spec://vibevm/a#r"], ["spec://vibevm/b#r"]);
        assert_eq!(table.sources["spec://vibevm/a#r"], ["spec://vibevm/c#r"]);
        assert_eq!(table.embeds["spec://vibevm/a#r"], ["spec://vibevm/d#r"]);
    }

    #[test]
    fn in_place_use_is_recorded_as_a_use_edge() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "see @spec://vibevm/b#r"),
            ("spec://vibevm/b#r", "leaf"),
        ]);
        let table = build_link_table(&seed(), &src).unwrap();
        assert_eq!(table.uses["spec://vibevm/a#r"], ["spec://vibevm/b#r"]);
    }

    #[test]
    fn traversal_is_cycle_safe() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "#use spec://vibevm/b#r"),
            ("spec://vibevm/b#r", "#use spec://vibevm/a#r"),
        ]);
        let table = build_link_table(&seed(), &src).unwrap();
        assert!(table.uses.contains_key("spec://vibevm/a#r"));
        assert!(table.uses.contains_key("spec://vibevm/b#r"));
    }

    #[test]
    fn render_is_stable_and_tab_separated() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "#use spec://vibevm/b#r"),
            ("spec://vibevm/b#r", "leaf"),
        ]);
        let table = build_link_table(&seed(), &src).unwrap();
        assert_eq!(
            table.render(),
            "use\tspec://vibevm/a#r\tspec://vibevm/b#r\n"
        );
    }

    #[test]
    fn an_unresolved_edge_is_reported() {
        let src = MockSource::new(&[("spec://vibevm/a#r", "#use spec://vibevm/missing#r")]);
        let err = build_link_table(&seed(), &src).unwrap_err();
        assert!(matches!(err, LinkTableError::Unresolved { .. }));
    }
}
