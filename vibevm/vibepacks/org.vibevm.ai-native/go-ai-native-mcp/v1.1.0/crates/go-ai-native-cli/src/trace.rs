//! `go-ai-native trace …` — traceability queries over the specmap
//! (PROP-014 §2.6), built fresh through the go-extract scanner so
//! explain answers for the tree as it is, never a stale artefact. The
//! render half is the neutral engine's — text, JSON, and the
//! ledger-cached prose are byte-identical in shape to the sibling
//! twins.

use std::path::Path;

use anyhow::Result;
use go_ai_native_specmap_scan::RecordsScanner;

pub fn run_trace_explain(root: &Path, target: &str, json: bool, prose: bool) -> Result<()> {
    let cfg = specmap_core::config::Config::load(root)?.unwrap_or_default();
    let extractor = go_ai_native_extract_bridge::materialise_extractor(root)?;
    let records = go_ai_native_extract_bridge::extract_tree(root, &extractor, None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let scanner = RecordsScanner::new(&records, "go");
    let map = specmap_core::index::build_with_scanner(root, &cfg, &scanner);
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
