//! `typescript-ai-native health` — the sweep fact-collector, TS shape
//! (`spec://org.vibevm.ai-native.core-ai-native/04-SWEEP-PLAYBOOK#collector`): the
//! questions the binary gates leave on the table, answered over the
//! SAME ts-tsc extraction the gates consume, so the numbers can never
//! drift from what the gates see.
//!
//! Sections: file-length early warning (the `[540, 600)` danger band —
//! files at 600 are landmines), the unsafe-set census (reasoned vs
//! unreasoned — every suppression is debt to watch), export
//! doc-example coverage per root, and the orphan backlog (untagged
//! exports the ratchet will block on once their root is gated).

use std::path::Path;

use anyhow::{Context, Result};

const BUDGET: u32 = 600;
const DANGER_FLOOR: u32 = 540;

pub fn run_health(root: &Path, out_rel: &str) -> Result<()> {
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let extractor = typescript_ai_native_extract_bridge::materialise_extractor(root)?;
    let records = typescript_ai_native_extract_bridge::extract_tree(root, &extractor, None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let ts_roots: Vec<String> = config
        .typescript
        .roots
        .iter()
        .map(|r| format!("{}/", r.trim_end_matches("/*").trim_end_matches('/')))
        .collect();
    let in_scope: Vec<&typescript_ai_native_extract_bridge::FileRecord> = records
        .iter()
        .filter(|r| ts_roots.iter().any(|p| r.file.starts_with(p.as_str())))
        .filter(|r| {
            !config
                .typescript
                .exclude_substrings
                .iter()
                .any(|s| r.file.contains(s.as_str()))
        })
        .collect();

    let mut over_budget: Vec<(String, u32)> = Vec::new();
    let mut danger: Vec<(String, u32)> = Vec::new();
    let mut unsafe_reasoned = 0usize;
    let mut unsafe_unreasoned = 0usize;
    let mut exports = 0usize;
    let mut exports_with_examples = 0usize;
    for record in &in_scope {
        for fact in &record.facts {
            match fact {
                typescript_ai_native_extract_bridge::RawFact::FileMetrics { lines } => {
                    if *lines > BUDGET {
                        over_budget.push((record.file.clone(), *lines));
                    } else if *lines >= DANGER_FLOOR {
                        danger.push((record.file.clone(), *lines));
                    }
                }
                typescript_ai_native_extract_bridge::RawFact::TsUnsafe { reason, .. } => {
                    if reason.is_some() {
                        unsafe_reasoned += 1;
                    } else {
                        unsafe_unreasoned += 1;
                    }
                }
                typescript_ai_native_extract_bridge::RawFact::Item {
                    is_exported,
                    has_doc_example,
                    ..
                } => {
                    if *is_exported && !record.in_test {
                        exports += 1;
                        if *has_doc_example {
                            exports_with_examples += 1;
                        }
                    }
                }
                typescript_ai_native_extract_bridge::RawFact::Import { .. } => {}
            }
        }
    }

    // The orphan backlog rides the specmap policy when one exists.
    let orphan_backlog = match specmap_core::config::Config::load(root)? {
        Some(cfg) => typescript_ai_native_specmap_scan::orphans(&records, &cfg)
            .into_iter()
            .map(|o| format!("{} ({}:{})", o.symbol, o.file, o.line))
            .collect(),
        None => Vec::new(),
    };

    danger.sort_by(|a, b| b.1.cmp(&a.1));
    over_budget.sort_by(|a, b| b.1.cmp(&a.1));

    let snapshot = serde_json::json!({
        "schema": 1,
        "collector": "typescript-ai-native health",
        "files_in_scope": in_scope.len(),
        "file_length": {
            "budget": BUDGET,
            "danger_floor": DANGER_FLOOR,
            "over_budget": over_budget.iter().map(|(f, l)| serde_json::json!({"file": f, "lines": l})).collect::<Vec<_>>(),
            "danger_band": danger.iter().map(|(f, l)| serde_json::json!({"file": f, "lines": l})).collect::<Vec<_>>(),
        },
        "unsafe_census": {
            "reasoned": unsafe_reasoned,
            "unreasoned": unsafe_unreasoned,
        },
        "export_doc_examples": {
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
         unsafe census {} reasoned / {} unreasoned; {}/{} exports carry examples; \
         orphan backlog {}. Snapshot at {out_rel}.",
        in_scope.len(),
        over_budget.len(),
        danger.len(),
        unsafe_reasoned,
        unsafe_unreasoned,
        exports_with_examples,
        exports,
        snapshot["orphan_backlog"].as_array().map_or(0, |a| a.len()),
    );
    Ok(())
}
