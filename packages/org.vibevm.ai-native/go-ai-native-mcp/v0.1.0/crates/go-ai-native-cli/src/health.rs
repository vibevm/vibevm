//! `go-ai-native health` — the sweep fact-collector, Go shape
//! (`spec://org.vibevm.ai-native/core-ai-native/04-SWEEP-PLAYBOOK#collector`):
//! the questions the binary gates leave on the table, answered over
//! the SAME go-extract extraction the gates consume, so the numbers
//! can never drift from what the gates see.
//!
//! Sections: file-length early warning (the `[540, 600)` danger band),
//! the ban census (reasoned vs unreasoned — every suppression and
//! deviation is testimony to watch), export Example coverage per
//! package (the package-grain join: `ExampleXxx` functions live in
//! sibling `_test.go` files, so coverage is computed here, not in the
//! extractor), and the orphan backlog (untagged exports the ratchet
//! will block on).

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#sweep");

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};

const BUDGET: u32 = 600;
const DANGER_FLOOR: u32 = 540;

fn dir_of(file: &str) -> &str {
    file.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

pub fn run_health(root: &Path, out_rel: &str) -> Result<()> {
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let extractor = go_ai_native_extract_bridge::materialise_extractor(root)?;
    let records = go_ai_native_extract_bridge::extract_tree(root, &extractor, None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let go_roots: Vec<String> = config
        .go
        .roots
        .iter()
        .map(|r| format!("{}/", r.trim_end_matches("/*").trim_end_matches('/')))
        .collect();
    let keeps_all = config.go.roots.iter().any(|r| r == ".");
    let in_scope: Vec<&go_ai_native_extract_bridge::FileRecord> = records
        .iter()
        .filter(|r| keeps_all || go_roots.iter().any(|p| r.file.starts_with(p.as_str())))
        .filter(|r| {
            !config
                .go
                .exclude_substrings
                .iter()
                .any(|s| r.file.contains(s.as_str()))
        })
        .collect();

    // The package-grain Example join: ExampleXxx functions declared in
    // a package's _test.go files cover the exported identifier Xxx of
    // the same directory.
    let mut examples_by_dir: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for record in &in_scope {
        if !record.in_test {
            continue;
        }
        for fact in &record.facts {
            if let go_ai_native_extract_bridge::RawFact::Item { kind, symbol, .. } = fact
                && kind == "func"
                && let Some(subject) = symbol.strip_prefix("Example")
                && !subject.is_empty()
            {
                examples_by_dir
                    .entry(dir_of(&record.file))
                    .or_default()
                    .insert(subject);
            }
        }
    }

    let mut over_budget: Vec<(String, u32)> = Vec::new();
    let mut danger: Vec<(String, u32)> = Vec::new();
    let mut census_reasoned = 0usize;
    let mut census_unreasoned = 0usize;
    let mut exports = 0usize;
    let mut exports_with_examples = 0usize;
    for record in &in_scope {
        for fact in &record.facts {
            match fact {
                go_ai_native_extract_bridge::RawFact::FileMetrics { lines } => {
                    if *lines > BUDGET {
                        over_budget.push((record.file.clone(), *lines));
                    } else if *lines >= DANGER_FLOOR {
                        danger.push((record.file.clone(), *lines));
                    }
                }
                go_ai_native_extract_bridge::RawFact::GoUnsafe { reason, .. } => {
                    if reason.is_some() {
                        census_reasoned += 1;
                    } else {
                        census_unreasoned += 1;
                    }
                }
                go_ai_native_extract_bridge::RawFact::Item {
                    symbol,
                    is_exported,
                    ..
                } => {
                    if *is_exported && !record.in_test {
                        exports += 1;
                        let covered = examples_by_dir
                            .get(dir_of(&record.file))
                            .is_some_and(|set| set.contains(symbol.as_str()));
                        if covered {
                            exports_with_examples += 1;
                        }
                    }
                }
                go_ai_native_extract_bridge::RawFact::Import { .. } => {}
            }
        }
    }

    // The orphan backlog rides the specmap policy when one exists.
    let orphan_backlog: Vec<String> = match specmap_core::config::Config::load(root)? {
        Some(cfg) => go_ai_native_specmap_scan::orphans(&records, &cfg)
            .into_iter()
            .map(|o| format!("{} ({}:{})", o.symbol, o.file, o.line))
            .collect(),
        None => Vec::new(),
    };

    danger.sort_by(|a, b| b.1.cmp(&a.1));
    over_budget.sort_by(|a, b| b.1.cmp(&a.1));

    let snapshot = serde_json::json!({
        "schema": 1,
        "collector": "go-ai-native health",
        "files_in_scope": in_scope.len(),
        "file_length": {
            "budget": BUDGET,
            "danger_floor": DANGER_FLOOR,
            "over_budget": over_budget.iter().map(|(f, l)| serde_json::json!({"file": f, "lines": l})).collect::<Vec<_>>(),
            "danger_band": danger.iter().map(|(f, l)| serde_json::json!({"file": f, "lines": l})).collect::<Vec<_>>(),
        },
        "ban_census": {
            "reasoned": census_reasoned,
            "unreasoned": census_unreasoned,
        },
        "export_examples": {
            "exports": exports,
            "with_examples": exports_with_examples,
        },
        "orphan_backlog": orphan_backlog,
    });

    let out_path = root.join(out_rel);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(&snapshot)?;
    text.push('\n');
    std::fs::write(&out_path, text).with_context(|| format!("writing {}", out_path.display()))?;

    eprintln!(
        "health: {} file(s) in scope; {} over budget, {} in the danger band; \
         ban census {} reasoned / {} unreasoned; {}/{} exports carry Examples; \
         orphan backlog {}. Snapshot at {out_rel}.",
        in_scope.len(),
        over_budget.len(),
        danger.len(),
        census_reasoned,
        census_unreasoned,
        exports_with_examples,
        exports,
        orphan_backlog.len(),
    );
    Ok(())
}
