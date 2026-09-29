//! `rust-ai-native trace …` — traceability queries over the specmap
//! (PROP-014 §2.6): the explain subgraph as text, JSON, or cached prose.

use std::path::Path;

use anyhow::Result;

pub fn run_trace_explain(root: &Path, target: &str, json: bool, prose: bool) -> Result<()> {
    // Build fresh in-memory: explain answers for the tree as it is,
    // never for a stale committed artefact.
    let cfg = specmap_core::config::Config::load(root)?.unwrap_or_default();
    let map = specmap_core::index::build(root, &cfg);
    if prose {
        let render = specmap_core::ledger::prose_explain(root, &map, target)?;
        print!("{}", render.text);
        let t = specmap_core::ledger::load_telemetry(root);
        eprintln!(
            "trace explain --prose: {} (epoch {}; ledger telemetry: {} hit(s), {} miss(es)).",
            if render.cached {
                "cache hit"
            } else {
                "computed fresh"
            },
            render.epoch.short(),
            t.hits,
            t.misses
        );
    } else if json {
        let v = specmap_core::explain::explain_json(&map, target)?;
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        print!("{}", specmap_core::explain::explain_text(&map, target)?);
    }
    Ok(())
}
