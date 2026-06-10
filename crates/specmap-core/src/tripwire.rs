//! Debt tripwires (BROWNFIELD-PROTOCOL §3): given a change set, list
//! the debt-registry entries whose watched paths fire. Warn-only by
//! contract — debt resurfaces exactly when it becomes relevant, it
//! never blocks.
//!
//! Two tripwire forms exist in the registry grammar: `touch:<glob>`
//! (matched here) and `rev:<spec-uri>` (needs specmap revisions —
//! evaluation lands with Phase 1; reported as not-yet-evaluable).

specmark::scope!("spec://vibevm/neworder/BROWNFIELD-PROTOCOL-v0.1#registries");

use anyhow::{Context, Result};
use serde::Deserialize;

/// One fired debt entry.
#[derive(Debug)]
pub struct Fired {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub disposition: String,
    /// (pattern, matched paths)
    pub hits: Vec<(String, Vec<String>)>,
    /// `rev:` tripwires present on the entry — not yet evaluable.
    pub unevaluated: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DebtFile {
    entries: Vec<DebtEntry>,
}

#[derive(Debug, Deserialize)]
struct DebtEntry {
    id: String,
    severity: String,
    title: String,
    disposition: String,
    #[serde(default)]
    tripwires: Vec<String>,
}

/// Match the change set against every non-fixed debt entry.
///
/// `changed` paths must be repo-relative, forward-slash.
pub fn evaluate(debt_json: &str, changed: &[String]) -> Result<Vec<Fired>> {
    let file: DebtFile = serde_json::from_str(debt_json).context("parsing debt.json")?;
    let mut fired = Vec::new();
    for entry in file.entries {
        // A fixed debt's tripwires retire with it; accepted/open/filed
        // entries keep watching.
        if entry.disposition == "fixed" {
            continue;
        }
        let mut hits: Vec<(String, Vec<String>)> = Vec::new();
        let mut unevaluated = Vec::new();
        for wire in &entry.tripwires {
            if let Some(glob_src) = wire.strip_prefix("touch:") {
                let pattern = glob::Pattern::new(glob_src)
                    .with_context(|| format!("{}: bad tripwire glob `{glob_src}`", entry.id))?;
                let matched: Vec<String> = changed
                    .iter()
                    .filter(|p| pattern.matches(p))
                    .cloned()
                    .collect();
                if !matched.is_empty() {
                    hits.push((wire.clone(), matched));
                }
            } else {
                unevaluated.push(wire.clone());
            }
        }
        if !hits.is_empty() {
            fired.push(Fired {
                id: entry.id,
                severity: entry.severity,
                title: entry.title,
                disposition: entry.disposition,
                hits,
                unevaluated,
            });
        }
    }
    Ok(fired)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEBT: &str = r#"{
      "entries": [
        { "id": "DBT-0001", "kind": "coverage-gap", "severity": "P1", "title": "registry path under-tested",
          "disposition": "filed",
          "tripwires": ["touch:crates/vibe-registry/src/**", "rev:spec://vibevm/x#req-a"] },
        { "id": "DBT-0015", "kind": "disputed-spec", "severity": "P2", "title": "fixed already",
          "disposition": "fixed",
          "tripwires": ["touch:spec/modules/vibe-resolver/**"] },
        { "id": "DBT-0017", "kind": "stale-doc", "severity": "P3", "title": "roadmap staleness",
          "disposition": "open",
          "tripwires": ["touch:ROADMAP.md"] }
      ]
    }"#;

    fn paths(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn touch_glob_fires_on_matching_change() {
        let fired = evaluate(
            DEBT,
            &paths(&["crates/vibe-registry/src/git_package_registry.rs"]),
        )
        .unwrap();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].id, "DBT-0001");
        assert_eq!(fired[0].hits.len(), 1);
        assert_eq!(fired[0].unevaluated, vec!["rev:spec://vibevm/x#req-a"]);
    }

    #[test]
    fn fixed_debts_do_not_fire() {
        let fired = evaluate(
            DEBT,
            &paths(&["spec/modules/vibe-resolver/PROP-003-dep-evolution.md"]),
        )
        .unwrap();
        assert!(fired.is_empty(), "{fired:?}");
    }

    #[test]
    fn exact_file_tripwire_fires() {
        let fired = evaluate(DEBT, &paths(&["ROADMAP.md"])).unwrap();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].id, "DBT-0017");
    }

    #[test]
    fn unrelated_changes_fire_nothing() {
        let fired = evaluate(DEBT, &paths(&["docs/commands/install.md"])).unwrap();
        assert!(fired.is_empty());
    }

    #[test]
    fn real_registry_parses_and_evaluates() {
        // Guard against the engine drifting from the real registry's
        // shape: the committed debt.json must always parse.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let json = std::fs::read_to_string(root.join("terraform/registry/debt.json")).unwrap();
        let fired = evaluate(&json, &paths(&["crates/vibe-registry/src/lib.rs"])).unwrap();
        // DBT-0001 watches crates/vibe-registry/src/** and is `filed`.
        assert!(fired.iter().any(|f| f.id == "DBT-0001"), "{fired:?}");
    }
}
