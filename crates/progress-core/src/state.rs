//! State projections — the dashboard's only food (PROP-043 §7.2).
//!
//! The vitrine computes nothing and parses no Markdown: everything it
//! shows is written here, atomically, after every tool step.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#state");

use crate::cache::{Cache, now_utc, write_atomic};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

pub const STATE_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignState {
    pub schema: u32,
    pub updated_at: String,
    pub campaign_id: String,
    /// The campaign plan letter currently active: "A" … "G".
    pub phase: String,
    pub wave: u32,
    pub counters: serde_json::Value,
}

/// Write all five state files from the cache (+ passthroughs that other
/// subsystems own: findings/tasks/docdebt are preserved when present,
/// seeded empty when absent — the dashboard always has valid JSON to eat).
pub fn write_state(state_dir: &Path, campaign_id: &str, phase: &str, cache: &Cache) -> Result<()> {
    let files: Vec<serde_json::Value> = cache
        .files
        .iter()
        .map(|(path, r)| {
            json!({
                "path": path,
                "effective": r.rollup.effective.map(|(st, s)| json!({"stage": st, "state": s})),
                "explicit": r.rollup.explicit.map(|(st, s)| json!({"stage": st, "state": s})),
                "markers": r.marker_count,
                "units": r.unit_count,
                "facts": r.rollup.fact_count,
                "unmarked": r.rollup.unmarked_facts,
                "issues": r.issue_count,
                "campaign": r.campaign,
            })
        })
        .collect();
    let total_facts: usize = cache.files.values().map(|r| r.rollup.fact_count).sum();
    let total_unmarked: usize = cache.files.values().map(|r| r.rollup.unmarked_facts).sum();
    let total_issues: usize = cache.files.values().map(|r| r.issue_count).sum();

    let corpus = json!({
        "schema": STATE_SCHEMA,
        "updated_at": now_utc(),
        "files": files,
    });
    write_atomic(
        &state_dir.join("corpus.json"),
        serde_json::to_string_pretty(&corpus)?.as_bytes(),
    )?;

    let campaign = CampaignState {
        schema: STATE_SCHEMA,
        updated_at: now_utc(),
        campaign_id: campaign_id.to_string(),
        phase: phase.to_string(),
        wave: 0,
        counters: json!({
            "files": cache.files.len(),
            "facts": total_facts,
            "unmarked": total_unmarked,
            "issues": total_issues,
        }),
    };
    write_atomic(
        &state_dir.join("campaign.json"),
        serde_json::to_string_pretty(&campaign)?.as_bytes(),
    )?;

    for (name, empty) in [
        (
            "findings.json",
            json!({"schema": STATE_SCHEMA, "updated_at": now_utc(), "findings": []}),
        ),
        (
            "tasks.json",
            json!({"schema": STATE_SCHEMA, "updated_at": now_utc(), "tasks": []}),
        ),
        (
            "docdebt.json",
            json!({"schema": STATE_SCHEMA, "updated_at": now_utc(), "cards": []}),
        ),
    ] {
        let p = state_dir.join(name);
        if !p.exists() {
            write_atomic(&p, serde_json::to_string_pretty(&empty)?.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_files_appear_and_are_valid_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut cache = Cache::default();
        let doc = crate::parse::parse_document("a.md", "@impl hello\n");
        let r = crate::rollup::rollup_doc(&doc);
        cache.upsert(&doc, &r);
        write_state(dir.path(), "progress-test", "A", &cache).expect("write");
        for f in [
            "corpus.json",
            "campaign.json",
            "findings.json",
            "tasks.json",
            "docdebt.json",
        ] {
            let text = std::fs::read_to_string(dir.path().join(f)).expect("read");
            let v: serde_json::Value = serde_json::from_str(&text).expect("json");
            assert_eq!(v["schema"], 1, "{f}");
        }
    }
}
