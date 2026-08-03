//! State projections — the dashboard's only food (PROP-043 §7.2).
//!
//! The vitrine computes nothing and parses no Markdown: everything it
//! shows is written here, atomically, after every tool step.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#state");

use crate::cache::{Cache, now_utc, write_atomic, write_if_changed};
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

pub const STATE_SCHEMA: u32 = 1;

/// A gate's last reported verdict. `stale` is what a gate holds once the
/// corpus moved under it and nobody re-ran it; `unknown` is a gate the
/// panel knows by name but has never heard from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateStatus {
    Green,
    Red,
    Stale,
    Unknown,
}

/// One row of the gate panel `campaign.json` carries. The verdict is
/// **reported in** by whoever ran the gate — this crate never measures it
/// (PROP-043 §2: the core runs no command and knows no floor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#state")]
pub struct GateRecord {
    /// The gate's identity — one record per name, later replaces earlier.
    pub name: String,
    pub status: GateStatus,
    /// When the reported run happened (RFC-3339 UTC).
    pub ran_at: String,
    /// Free text pinned to the verdict: the failing test, the exit code,
    /// the reason it is stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignState {
    pub schema: u32,
    pub updated_at: String,
    pub campaign_id: String,
    /// The campaign plan letter currently active: "A" … "G".
    pub phase: String,
    pub wave: u32,
    pub counters: serde_json::Value,
    /// The gate panel. Absent from the JSON while empty, so a projection
    /// written before any gate was recorded is byte-identical to the
    /// pre-panel shape and no existing consumer sees a change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<GateRecord>,
}

/// Read `campaign.json` back into its struct.
fn read_campaign(path: &Path) -> Result<CampaignState> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Write all five state files from the cache (+ passthroughs that other
/// subsystems own: findings/tasks/docdebt are preserved when present,
/// seeded empty when absent — the dashboard always has valid JSON to eat).
///
/// Returns one entry per projection, `true` where this call actually wrote
/// it. A projection whose content is already on disk is left alone —
/// untouched and unfsync'd — and says so, because a skip nobody can see is
/// an optimisation nobody can debug (DRIFT-017 §4.3).
pub fn write_state(
    state_dir: &Path,
    campaign_id: &str,
    phase: &str,
    cache: &Cache,
) -> Result<BTreeMap<String, bool>> {
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
                // The view, not the record: the verdict tally the campaign
                // used to store beside its verdicts is computed here, on
                // every write, so the dashboard still reads a `summary`
                // and can never read a stale one (DRIFT-033, F-077).
                "campaign": r.campaign_view(),
            })
        })
        .collect();
    let total_facts: usize = cache.files.values().map(|r| r.rollup.fact_count).sum();
    let total_unmarked: usize = cache.files.values().map(|r| r.rollup.unmarked_facts).sum();
    let total_issues: usize = cache.files.values().map(|r| r.issue_count).sum();

    let mut written = BTreeMap::new();
    let corpus = json!({
        "schema": STATE_SCHEMA,
        "updated_at": now_utc(),
        "files": files,
    });
    written.insert(
        "corpus.json".to_string(),
        write_if_changed(
            &state_dir.join("corpus.json"),
            &serde_json::to_string_pretty(&corpus)?,
        )?,
    );

    // A scan re-derives the counters; it must never erase the panel a caller
    // reported in. An unreadable projection degrades to an empty panel rather
    // than wedging every scan — `campaign.json` is a derived artifact
    // (PROP-043 §7.5) and `record_gate` is the path that fails loudly.
    let campaign_path = state_dir.join("campaign.json");
    let gates = read_campaign(&campaign_path)
        .map(|prev| prev.gates)
        .unwrap_or_default();

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
        gates,
    };
    written.insert(
        "campaign.json".to_string(),
        write_if_changed(&campaign_path, &serde_json::to_string_pretty(&campaign)?)?,
    );

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
        // Seeded, never refreshed: these three belong to other subsystems,
        // so "already there" has always meant "leave it alone" here. The
        // skip below is the same answer arrived at the same way — an
        // absent file is the only one this call has anything to say about.
        let p = state_dir.join(name);
        let seed = !p.exists();
        if seed {
            write_atomic(&p, serde_json::to_string_pretty(&empty)?.as_bytes())?;
        }
        written.insert(name.to_string(), seed);
    }
    Ok(written)
}

/// Record one gate's verdict into `campaign.json`: the entry with the same
/// `name` is replaced, a new name is appended, and the file is rewritten
/// atomically.
///
/// Gates are **recorded, never computed here** — the caller runs the real
/// gate (a CI step, a local script) and reports what it found. This function
/// spawns nothing and reads nothing outside `state_dir`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#state")]
pub fn record_gate(state_dir: &Path, gate: GateRecord) -> Result<()> {
    let path = state_dir.join("campaign.json");
    if !path.exists() {
        bail!(
            "no state projection at {} — the gate panel lives in that file; \
             run `vibe progress scan` first to write it",
            path.display()
        );
    }
    let mut state = read_campaign(&path)?;
    match state.gates.iter_mut().find(|g| g.name == gate.name) {
        Some(slot) => *slot = gate,
        None => state.gates.push(gate),
    }
    state.updated_at = now_utc();
    write_atomic(&path, serde_json::to_string_pretty(&state)?.as_bytes())
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

    /// F-077's other half, on the seam it matters at: the dashboard is
    /// still handed a per-file `summary`, and it is handed one the cache
    /// does not carry.
    ///
    /// That is the whole of the change from a consumer's side — the same
    /// key with the same numbers, recomputed at every write instead of
    /// copied forward from whenever it was last typed. The projection is
    /// rewritten on every scan, so the count cannot outlive the verdicts
    /// it counts; the stored field could, and the only reason it never
    /// did is that nobody had edited a verdict by hand yet.
    #[test]
    fn the_projection_carries_a_computed_summary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut cache = Cache::default();
        let doc = crate::parse::parse_document("a.md", "@impl hello\n");
        cache.upsert(&doc, &crate::rollup::rollup_doc(&doc));
        cache
            .files
            .get_mut("a.md")
            .expect("the record just upserted")
            .campaign
            .insert(
                "verdicts".into(),
                json!({"a1": {"v": "confirmed"}, "a2": {"v": "drift"}, "a3": {"v": "confirmed"}}),
            );

        write_state(dir.path(), "progress-test", "A", &cache).expect("write");
        let corpus: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("corpus.json")).expect("read"),
        )
        .expect("json");

        assert_eq!(
            corpus["files"][0]["campaign"]["summary"],
            json!({"confirmed": 2, "drift": 1}),
            "the reader gets the count"
        );
        assert!(
            !cache.files["a.md"].campaign.contains_key("summary"),
            "and the cache it was computed from never held one"
        );
    }

    fn gate(name: &str, status: GateStatus, detail: Option<&str>) -> GateRecord {
        GateRecord {
            name: name.into(),
            status,
            ran_at: "2026-07-25T00:00:00Z".into(),
            detail: detail.map(str::to_string),
        }
    }

    /// Until a gate is recorded, `campaign.json` is byte-identical to the
    /// shape it carried before the panel existed — no `gates` key at all,
    /// so no existing consumer of the file sees a change.
    #[test]
    fn gates_absent_serialises_to_no_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_state(dir.path(), "progress-test", "A", &Cache::default()).expect("write");
        let text = std::fs::read_to_string(dir.path().join("campaign.json")).expect("read");
        let v: serde_json::Value = serde_json::from_str(&text).expect("json");
        let ts = v["updated_at"].as_str().expect("updated_at");
        let expected = format!(
            r#"{{
  "schema": 1,
  "updated_at": "{ts}",
  "campaign_id": "progress-test",
  "phase": "A",
  "wave": 0,
  "counters": {{
    "files": 0,
    "facts": 0,
    "unmarked": 0,
    "issues": 0
  }}
}}"#
        );
        assert_eq!(text, expected, "no gate recorded ⇒ the pre-panel bytes");
    }

    /// One name, one row: recording `floor` twice leaves the later verdict
    /// alone in the panel.
    #[test]
    fn record_gate_appends_then_replaces() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_state(dir.path(), "progress-test", "A", &Cache::default()).expect("write");
        record_gate(dir.path(), gate("floor", GateStatus::Green, None)).expect("green");
        record_gate(dir.path(), gate("check", GateStatus::Green, None)).expect("other gate");
        record_gate(
            dir.path(),
            gate("floor", GateStatus::Red, Some("cli_pkg_cycle")),
        )
        .expect("red");

        let text = std::fs::read_to_string(dir.path().join("campaign.json")).expect("read");
        let state: CampaignState = serde_json::from_str(&text).expect("state");
        let floor: Vec<&GateRecord> = state.gates.iter().filter(|g| g.name == "floor").collect();
        assert_eq!(floor.len(), 1, "the later record replaces the earlier");
        assert_eq!(floor[0].status, GateStatus::Red);
        assert_eq!(floor[0].detail.as_deref(), Some("cli_pkg_cycle"));
        assert_eq!(state.gates.len(), 2, "a different name appends");
    }

    /// A scan re-derives the counters — it must never erase the panel.
    #[test]
    fn write_state_preserves_gates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = Cache::default();
        write_state(dir.path(), "progress-test", "A", &cache).expect("write");
        record_gate(dir.path(), gate("floor", GateStatus::Red, Some("F-055"))).expect("record");

        write_state(dir.path(), "progress-test", "B", &cache).expect("rescan");

        let text = std::fs::read_to_string(dir.path().join("campaign.json")).expect("read");
        let state: CampaignState = serde_json::from_str(&text).expect("state");
        assert_eq!(state.phase, "B", "the scan still refreshed the projection");
        assert_eq!(
            state.gates,
            vec![gate("floor", GateStatus::Red, Some("F-055"))]
        );
    }

    /// The state dir must exist first: the error names the file and the
    /// command that writes it.
    #[test]
    fn record_gate_without_state_names_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = record_gate(dir.path(), gate("floor", GateStatus::Green, None))
            .expect_err("no campaign.json ⇒ error");
        let msg = format!("{err:#}");
        assert!(msg.contains("campaign.json"), "{msg}");
        assert!(msg.contains("vibe progress scan"), "{msg}");
    }
}
