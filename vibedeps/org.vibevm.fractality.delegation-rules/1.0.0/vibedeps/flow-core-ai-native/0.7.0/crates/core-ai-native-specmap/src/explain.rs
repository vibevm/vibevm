//! `trace explain` — render the traceability subgraph around one
//! target (PROP-014 §2.6). The `--text` rendering is deterministic and
//! structured; `--json` emits the raw subgraph for agents. Fully
//! useful without an LLM by contract — prose rendering is a later,
//! separate presentation layer.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use crate::generated::specmap::{Edge, SpecUnit, Specmap};
use anyhow::{Result, bail};

fn verb_str(e: &Edge) -> &'static str {
    use crate::generated::specmap::EdgeVerb::*;
    match e.verb {
        Implements => "implements",
        Verifies => "verifies",
        Documents => "documents",
        Deviates => "deviates",
        Informs => "informs",
    }
}

fn unit_line(u: &SpecUnit) -> String {
    let kind = u
        .kind
        .as_deref()
        .map(|k| {
            use crate::generated::specmap::SpecUnitKind::*;
            match k {
                Prop => "prop",
                Req => "req",
                Design => "design",
                Guide => "guide",
            }
        })
        .unwrap_or("unmarked");
    let rev = u
        .revision
        .as_deref()
        .map(|r| format!(" r{r}"))
        .unwrap_or_default();
    let status = u
        .status
        .as_deref()
        .map(|s| {
            use crate::generated::specmap::SpecUnitStatus::*;
            match s {
                Planned => " [PLANNED]".to_string(),
                Disputed => format!(
                    " [DISPUTED{}]",
                    u.disputes
                        .as_deref()
                        .map(|d| format!(" ↔ #{d}"))
                        .unwrap_or_default()
                ),
            }
        })
        .unwrap_or_default();
    format!(
        "{kind}{rev}{status} — {} ({}:{})",
        u.heading, u.file, u.line
    )
}

fn edge_suffix(map: &Specmap, e: &Edge) -> String {
    let mut s = String::new();
    if let Some(p) = e.pinnedR.as_deref() {
        s.push_str(&format!(" (pinned r{p})"));
    }
    if map
        .suspects
        .iter()
        .any(|x| x.fromSymbol == e.fromSymbol && x.uri == e.uri)
    {
        let cur = map
            .suspects
            .iter()
            .find(|x| x.fromSymbol == e.fromSymbol && x.uri == e.uri)
            .map(|x| x.currentR)
            .unwrap_or_default();
        s.push_str(&format!(" [SUSPECT: unit is at r{cur}]"));
    }
    if let Some(reason) = e.reason.as_deref() {
        s.push_str(&format!("\n      deviation: {reason}"));
    }
    s
}

/// Render the subgraph around a `spec://` URI: the unit, every edge
/// into it, suspects.
fn explain_unit(map: &Specmap, uri: &str) -> Result<String> {
    let units: Vec<&SpecUnit> = map.specUnits.iter().filter(|u| u.uri == uri).collect();
    if units.is_empty() {
        bail!("no spec unit with URI `{uri}` in the index");
    }
    let mut out = String::new();
    for u in units {
        out.push_str(&format!("spec unit {uri}\n  {}\n", unit_line(u)));
        out.push_str(&format!("  hash {}\n", &u.contentHash));
    }
    let edges: Vec<&Edge> = map.edges.iter().filter(|e| e.uri == uri).collect();
    if edges.is_empty() {
        out.push_str("  edges: none (uncovered unit)\n");
    } else {
        out.push_str("  edges in:\n");
        for e in edges {
            out.push_str(&format!(
                "    {} ← `{}` ({}:{}){}\n",
                verb_str(e),
                e.fromSymbol,
                e.file,
                e.line,
                edge_suffix(map, e)
            ));
        }
    }
    Ok(out)
}

/// Render the subgraph around a code symbol: the item, its edges out,
/// each target unit, and sibling edges into those units (coverage
/// context — e.g. which tests verify the same contract).
fn explain_symbol(map: &Specmap, target: &str) -> Result<String> {
    let exact: Vec<_> = map
        .codeItems
        .iter()
        .filter(|i| i.symbol == target)
        .collect();
    let items = if !exact.is_empty() {
        exact
    } else {
        let suffix: Vec<_> = map
            .codeItems
            .iter()
            .filter(|i| i.symbol.ends_with(target))
            .collect();
        match suffix.len() {
            0 => bail!(
                "no tagged code item matches `{target}` (neither exactly nor as a suffix); \
                 untagged items are outside the migrated frontier — facts only"
            ),
            1 => suffix,
            _ => {
                let mut candidates: Vec<&str> = suffix.iter().map(|i| i.symbol.as_str()).collect();
                candidates.sort();
                bail!(
                    "`{target}` is ambiguous; candidates:\n  {}",
                    candidates.join("\n  ")
                );
            }
        }
    };

    let mut out = String::new();
    for item in items {
        out.push_str(&format!(
            "code item `{}`\n  {} in {} ({}:{})\n",
            item.symbol, item.itemKind, item.crateName, item.file, item.line
        ));
        let edges: Vec<&Edge> = map
            .edges
            .iter()
            .filter(|e| e.fromSymbol == item.symbol)
            .collect();
        if edges.is_empty() {
            out.push_str("  edges: none\n");
            continue;
        }
        for e in &edges {
            out.push_str(&format!(
                "  --{}--> {}{}\n",
                verb_str(e),
                e.uri,
                edge_suffix(map, e)
            ));
            for u in map.specUnits.iter().filter(|u| u.uri == e.uri) {
                out.push_str(&format!("      unit: {}\n", unit_line(u)));
            }
            // Sibling edges: who else touches this unit.
            for s in map
                .edges
                .iter()
                .filter(|s| s.uri == e.uri && s.fromSymbol != item.symbol)
            {
                out.push_str(&format!(
                    "      also: {} ← `{}` ({}:{})\n",
                    verb_str(s),
                    s.fromSymbol,
                    s.file,
                    s.line
                ));
            }
        }
    }
    Ok(out)
}

/// `--text` rendering for a symbol or a `spec://` URI.
pub fn explain_text(map: &Specmap, target: &str) -> Result<String> {
    if target.starts_with("spec://") {
        explain_unit(map, target)
    } else {
        explain_symbol(map, target)
    }
}

/// `--json`: the raw subgraph — the items, edges, units and suspects
/// reachable from the target in one hop.
pub fn explain_json(map: &Specmap, target: &str) -> Result<serde_json::Value> {
    let (symbols, uris): (Vec<String>, Vec<String>) = if target.starts_with("spec://") {
        let symbols = map
            .edges
            .iter()
            .filter(|e| e.uri == target)
            .map(|e| e.fromSymbol.clone())
            .collect();
        (symbols, vec![target.to_string()])
    } else {
        let symbols: Vec<String> = map
            .codeItems
            .iter()
            .filter(|i| i.symbol == target || i.symbol.ends_with(target))
            .map(|i| i.symbol.clone())
            .collect();
        if symbols.is_empty() {
            bail!("no tagged code item matches `{target}`");
        }
        let uris = map
            .edges
            .iter()
            .filter(|e| symbols.contains(&e.fromSymbol))
            .map(|e| e.uri.clone())
            .collect();
        (symbols, uris)
    };

    let items: Vec<_> = map
        .codeItems
        .iter()
        .filter(|i| symbols.contains(&i.symbol))
        .map(|i| {
            serde_json::json!({
                "symbol": i.symbol, "item_kind": i.itemKind,
                "crate_name": i.crateName, "file": i.file, "line": i.line,
            })
        })
        .collect();
    let edges: Vec<_> = map
        .edges
        .iter()
        .filter(|e| symbols.contains(&e.fromSymbol) || uris.contains(&e.uri))
        .map(|e| {
            serde_json::json!({
                "from_symbol": e.fromSymbol, "verb": verb_str(e), "uri": e.uri,
                "pinned_r": e.pinnedR.as_deref(), "reason": e.reason.as_deref(),
                "file": e.file, "line": e.line,
            })
        })
        .collect();
    let units: Vec<_> = map
        .specUnits
        .iter()
        .filter(|u| uris.contains(&u.uri))
        .map(|u| {
            serde_json::json!({
                "uri": u.uri, "heading": u.heading,
                "revision": u.revision.as_deref(),
                "content_hash": u.contentHash, "file": u.file, "line": u.line,
            })
        })
        .collect();
    let suspects: Vec<_> = map
        .suspects
        .iter()
        .filter(|s| symbols.contains(&s.fromSymbol) || uris.contains(&s.uri))
        .map(|s| {
            serde_json::json!({
                "from_symbol": s.fromSymbol, "uri": s.uri,
                "pinned_r": s.pinnedR, "current_r": s.currentR,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "target": target,
        "items": items,
        "edges": edges,
        "units": units,
        "suspects": suspects,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::specmap::{
        CodeItem, EdgeProvenance, EdgeVerb, SpecUnitKind, SpecUnitStatus,
    };

    const GRAMMAR: &str = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar";
    const COMPOSITION: &str =
        "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition";

    fn fixture() -> Specmap {
        Specmap {
            schema: 2,
            specUnits: vec![
                SpecUnit {
                    uri: GRAMMAR.to_string(),
                    docPath: "modules/vibe-resolver/PROP-003".to_string(),
                    file: "spec/modules/vibe-resolver/PROP-003-dep-evolution.md".to_string(),
                    anchor: "req-conditional-grammar".to_string(),
                    heading: "The predicate grammar".to_string(),
                    contentHash: "sha256:aa".to_string(),
                    line: 400,
                    kind: Some(Box::new(SpecUnitKind::Req)),
                    revision: Some(Box::new(1)),
                    status: None,
                    disputes: None,
                },
                SpecUnit {
                    uri: COMPOSITION.to_string(),
                    docPath: "modules/vibe-resolver/PROP-003".to_string(),
                    file: "spec/modules/vibe-resolver/PROP-003-dep-evolution.md".to_string(),
                    anchor: "req-conditional-composition".to_string(),
                    heading: "Boolean composition over predicates".to_string(),
                    contentHash: "sha256:bb".to_string(),
                    line: 420,
                    kind: Some(Box::new(SpecUnitKind::Req)),
                    revision: Some(Box::new(1)),
                    status: Some(Box::new(SpecUnitStatus::Planned)),
                    disputes: None,
                },
            ],
            codeItems: vec![CodeItem {
                symbol: "vibe_resolver::conditional::ConditionalPredicate::parse".to_string(),
                itemKind: "fn".to_string(),
                crateName: "vibe-resolver".to_string(),
                file: "crates/vibe-resolver/src/conditional.rs".to_string(),
                line: 32,
            }],
            edges: vec![
                Edge {
                    fromSymbol: "vibe_resolver::conditional::ConditionalPredicate::parse"
                        .to_string(),
                    verb: EdgeVerb::Implements,
                    uri: GRAMMAR.to_string(),
                    provenance: EdgeProvenance::Authored,
                    file: "crates/vibe-resolver/src/conditional.rs".to_string(),
                    line: 30,
                    pinnedR: Some(Box::new(1)),
                    reason: None,
                },
                Edge {
                    fromSymbol: "vibe_resolver::conditional::ConditionalPredicate::parse"
                        .to_string(),
                    verb: EdgeVerb::Deviates,
                    uri: COMPOSITION.to_string(),
                    provenance: EdgeProvenance::Authored,
                    file: "crates/vibe-resolver/src/conditional.rs".to_string(),
                    line: 31,
                    pinnedR: Some(Box::new(1)),
                    reason: Some(Box::new("composition unimplemented".to_string())),
                },
                Edge {
                    fromSymbol: "vibe_resolver::conditional::tests::parses_simple".to_string(),
                    verb: EdgeVerb::Verifies,
                    uri: GRAMMAR.to_string(),
                    provenance: EdgeProvenance::Authored,
                    file: "crates/vibe-resolver/src/conditional.rs".to_string(),
                    line: 80,
                    pinnedR: Some(Box::new(1)),
                    reason: None,
                },
            ],
            suspects: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn symbol_view_renders_edges_units_and_siblings() {
        let map = fixture();
        let text = explain_text(
            &map,
            "vibe_resolver::conditional::ConditionalPredicate::parse",
        )
        .unwrap();
        assert!(text.contains("--implements-->"), "{text}");
        assert!(text.contains("--deviates-->"), "{text}");
        assert!(text.contains("[PLANNED]"), "{text}");
        assert!(
            text.contains("deviation: composition unimplemented"),
            "{text}"
        );
        assert!(
            text.contains("also: verifies ← `vibe_resolver::conditional::tests::parses_simple`"),
            "{text}"
        );
    }

    #[test]
    fn suffix_resolution_finds_the_unique_item() {
        let map = fixture();
        let text = explain_text(&map, "ConditionalPredicate::parse").unwrap();
        assert!(text.contains("code item"), "{text}");
    }

    #[test]
    fn unit_view_lists_incoming_edges() {
        let map = fixture();
        let text = explain_text(&map, GRAMMAR).unwrap();
        assert!(text.contains("edges in:"), "{text}");
        assert!(text.contains("implements ←"), "{text}");
        assert!(text.contains("verifies ←"), "{text}");
    }

    #[test]
    fn unknown_targets_error_clearly() {
        let map = fixture();
        assert!(explain_text(&map, "no::such::thing").is_err());
        assert!(explain_text(&map, "spec://vibevm/x#nope").is_err());
    }

    #[test]
    fn json_subgraph_has_all_four_tables() {
        let map = fixture();
        let v = explain_json(&map, "ConditionalPredicate::parse").unwrap();
        assert_eq!(v["items"].as_array().unwrap().len(), 1);
        assert_eq!(v["edges"].as_array().unwrap().len(), 3);
        assert_eq!(v["units"].as_array().unwrap().len(), 2);
    }
}
