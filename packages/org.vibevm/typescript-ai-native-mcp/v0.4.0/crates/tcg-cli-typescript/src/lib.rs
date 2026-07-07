//! tcg-cli-typescript — the enrichment layer and the delivery surface
//! of the agentic type oracle (TCG-PROTOCOL-v0.1 §3).
//!
//! ALL policy interpretation lives here, once: `conform.toml` loading,
//! rule assembly (through `conform_cli_typescript::build_rules` — the
//! gate's own set), baseline awareness, and advice strings. The node
//! side ships facts; this layer turns them into the same findings the
//! gate would raise, flagged against the same frozen baseline.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Serialize;
use tcg_oracle_bridge::{OracleTransport, Position, SystemOracle, ValidateResult, to_file_record};

pub mod bench;
mod serve;

pub use bench::run_bench;
pub use serve::run_serve;

/// Cold init can compile a real program; queries after that are
/// milliseconds. One budget covers both (TCG-ORACLE §7 posts the
/// targets; the budget is an order above them).
pub const ORACLE_TIMEOUT: Duration = Duration::from_secs(30);

/// The policy bundle the enrichment layer answers from — the SAME
/// objects the conform gate builds.
pub struct Policy {
    pub config: conform_core::Config,
    pub baseline: Vec<String>,
}

impl Policy {
    /// Load `conform.toml` (or the engine default) + the frozen TS
    /// baseline fingerprints (absent file = empty set — nothing is
    /// sanctioned by default).
    pub fn load(root: &Path) -> Result<Self> {
        let (config, _origin) = conform_core::Config::load_or_default(root)
            .map_err(|e| anyhow::anyhow!("loading conform.toml: {e}"))?;
        let baseline_path = root.join(conform_cli_typescript::DEFAULT_TS_BASELINE);
        let baseline = if baseline_path.exists() {
            conform_core::baseline::load(&baseline_path)
                .map_err(|e| anyhow::anyhow!("loading TS baseline: {e}"))?
                .findings
        } else {
            Vec::new()
        };
        Ok(Self { config, baseline })
    }

    fn cells_dir(&self) -> Option<&str> {
        self.config.typescript.cells_dir.as_deref()
    }

    fn seam(&self) -> &str {
        &self.config.typescript.seam
    }
}

/// One conform finding as the oracle reports it: the gate's rule id and
/// message, plus whether the project's frozen baseline sanctions it.
#[derive(Debug, Clone, Serialize)]
pub struct ConformFindingOut {
    pub rule: String,
    pub message: String,
    pub line: u32,
    pub baselined: bool,
}

/// The §3 enrichment of a validate response.
#[derive(Debug, Serialize)]
pub struct EnrichedValidate {
    #[serde(flatten)]
    pub raw: ValidateResult,
    pub conform_findings: Vec<ConformFindingOut>,
    pub advice: Vec<String>,
}

/// Run the gate's rule set over one validated file's facts and merge
/// the findings into the response (TCG-PROTOCOL §3). Pure over its
/// inputs — unit-tested with no node anywhere near.
pub fn enrich_validate(policy: &Policy, file: &str, raw: ValidateResult) -> EnrichedValidate {
    let record = to_file_record(file, &raw);
    let facts = ts_extract_bridge::conform_facts(&record);
    let source = conform_core::SourceFacts {
        file: file.to_string(),
        crate_name: String::new(),
        facts,
    };
    let owned = conform_cli_typescript::build_rules(&policy.config);
    let rule_refs: Vec<&dyn conform_core::Rule> = owned.iter().map(|r| r.as_ref()).collect();
    let findings = conform_core::check(&rule_refs, &[source], None);

    let mut advice: Vec<String> = Vec::new();
    let conform_findings: Vec<ConformFindingOut> = findings
        .iter()
        .map(|f| {
            let baselined = policy.baseline.iter().any(|fp| fp == &f.fingerprint);
            if !baselined && f.rule == "ts-unsafe-in-domain" {
                advice.push(
                    "an unsafe-set form in domain code: prefer `unknown` + a runtime \
                     validator, a checked `as` after a guard, or `@ts-expect-error -- \
                     reason` (spec://typescript-ai-native-lang/guide s8)"
                        .to_string(),
                );
            }
            if !baselined && f.rule == "ts-cell-isolation" {
                advice.push(
                    "cells import each other only through their seam module \
                     (spec://typescript-ai-native-lang/guide s3)"
                        .to_string(),
                );
            }
            ConformFindingOut {
                rule: f.rule.to_string(),
                message: f.message.clone(),
                line: f.line,
                baselined,
            }
        })
        .collect();
    advice.dedup();

    EnrichedValidate {
        raw,
        conform_findings,
        advice,
    }
}

/// `line:character` → protocol position (1-based line, 0-based char).
pub fn parse_position(s: &str) -> Result<Position> {
    let (l, c) = s
        .split_once(':')
        .with_context(|| format!("position `{s}` is not `line:character`"))?;
    Ok(Position {
        line: l.trim().parse().with_context(|| format!("line in `{s}`"))?,
        character: c
            .trim()
            .parse()
            .with_context(|| format!("character in `{s}`"))?,
    })
}

fn read_content(content_from: Option<&str>) -> Result<Option<String>> {
    match content_from {
        None => Ok(None),
        Some("-") => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("reading stdin")?;
            Ok(Some(buf))
        }
        Some(path) => Ok(Some(
            std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?,
        )),
    }
}

fn spawn_ready(root: &Path, policy: &Policy) -> Result<SystemOracle> {
    let mut oracle = SystemOracle::spawn(root, ORACLE_TIMEOUT)?;
    let init = oracle.init(root, policy.cells_dir(), policy.seam())?;
    eprintln!(
        "tcg-typescript: oracle up — typescript {}, {} root file(s), config {}",
        init.ts_version, init.root_files, init.config_file
    );
    Ok(oracle)
}

/// One-shot validate. Exit 0 = no error diagnostics AND no non-baselined
/// findings; exit 1 otherwise (script-composable, like the gate).
pub fn run_validate(
    root: &Path,
    file: &str,
    content_from: Option<&str>,
    json: bool,
) -> Result<i32> {
    let policy = Policy::load(root)?;
    let content = read_content(content_from)?;
    let mut oracle = spawn_ready(root, &policy)?;
    let raw = oracle.validate(file, content.as_deref())?;
    let enriched = enrich_validate(&policy, file, raw);
    let errors = enriched
        .raw
        .diagnostics
        .iter()
        .filter(|d| d.category == "error")
        .count();
    let new_findings = enriched
        .conform_findings
        .iter()
        .filter(|f| !f.baselined)
        .count();
    if json {
        println!("{}", serde_json::to_string_pretty(&enriched)?);
    } else {
        for d in &enriched.raw.diagnostics {
            println!(
                "{}:{}:{} TS{} {}",
                file, d.line, d.character, d.code, d.message
            );
        }
        for f in &enriched.conform_findings {
            println!(
                "{}:{} {} {}{}",
                file,
                f.line,
                f.rule,
                f.message,
                if f.baselined { " [baselined]" } else { "" }
            );
        }
        for a in &enriched.advice {
            println!("advice: {a}");
        }
        println!(
            "tcg-typescript validate: {} diagnostic(s) ({errors} error(s)), \
             {} finding(s) ({new_findings} new), degraded={}",
            enriched.raw.diagnostics.len(),
            enriched.conform_findings.len(),
            enriched.raw.degraded,
        );
    }
    let _ = oracle.shutdown();
    Ok(i32::from(errors > 0 || new_findings > 0))
}

/// One-shot scope.
pub fn run_scope(root: &Path, file: &str, position: Option<&str>, json: bool) -> Result<i32> {
    let policy = Policy::load(root)?;
    let pos = position.map(parse_position).transpose()?;
    let mut oracle = spawn_ready(root, &policy)?;
    let result = oracle.scope(file, pos)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "cell: {}  seam: {}",
            result.cell.as_deref().unwrap_or("-"),
            result.seam_file.as_deref().unwrap_or("-")
        );
        for b in &result.branded {
            println!(
                "branded: {} (at {}{})",
                b.name,
                b.seam,
                if b.heuristic { ", heuristic" } else { "" }
            );
        }
        println!("{} in-scope symbol(s); first 20:", result.symbols.len());
        for s in result.symbols.iter().take(20) {
            println!("  {} [{}]", s.name, s.kind);
        }
    }
    let _ = oracle.shutdown();
    Ok(0)
}

/// One-shot complete.
#[allow(clippy::too_many_arguments)]
pub fn run_complete(
    root: &Path,
    file: &str,
    position: &str,
    prefix: Option<&str>,
    max: u64,
    content_from: Option<&str>,
    json: bool,
) -> Result<i32> {
    let policy = Policy::load(root)?;
    let pos = parse_position(position)?;
    let content = read_content(content_from)?;
    let mut oracle = spawn_ready(root, &policy)?;
    let result = oracle.complete(file, pos, content.as_deref(), prefix, max)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        for e in &result.entries {
            println!(
                "{} [{}]{}{}",
                e.name,
                e.kind,
                if e.type_text.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", e.type_text)
                },
                if e.unsafe_ {
                    "  [UNSAFE: any-typed]"
                } else {
                    ""
                }
            );
        }
        println!("{} entr(ies)", result.entries.len());
    }
    let _ = oracle.shutdown();
    Ok(0)
}

/// One-shot quick info.
pub fn run_type(root: &Path, file: &str, position: &str, json: bool) -> Result<i32> {
    let policy = Policy::load(root)?;
    let pos = parse_position(position)?;
    let mut oracle = spawn_ready(root, &policy)?;
    let result = oracle.quick_info(file, pos, None)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.display);
        if !result.documentation.is_empty() {
            println!("{}", result.documentation);
        }
    }
    let _ = oracle.shutdown();
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcg_oracle_bridge::Diagnostic;
    use ts_extract_bridge::{RawFact, RawMarker};

    fn policy_with(cells: Option<&str>, baseline: Vec<String>) -> Policy {
        let mut config = conform_core::Config::default();
        config.typescript.cells_dir = cells.map(str::to_string);
        Policy { config, baseline }
    }

    fn validate_result(facts: Vec<RawFact>) -> ValidateResult {
        ValidateResult {
            diagnostics: vec![Diagnostic {
                code: 2322,
                category: "error".to_string(),
                message: "type mismatch".to_string(),
                line: 4,
                character: 2,
            }],
            facts,
            markers: Vec::<RawMarker>::new(),
            degraded: false,
        }
    }

    #[test]
    fn enrichment_raises_the_gates_findings_and_advice() {
        let policy = policy_with(None, Vec::new());
        let raw = validate_result(vec![
            RawFact::FileMetrics { lines: 10 },
            RawFact::TsUnsafe {
                kind: "any_type".to_string(),
                line: 3,
                reason: None,
            },
        ]);
        let enriched = enrich_validate(&policy, "src/cells/a/index.ts", raw);
        assert_eq!(enriched.conform_findings.len(), 1);
        let f = &enriched.conform_findings[0];
        assert_eq!(f.rule, "ts-unsafe-in-domain");
        assert!(!f.baselined);
        assert!(enriched.advice.iter().any(|a| a.contains("guide s8")));
    }

    #[test]
    fn baselined_findings_are_flagged_not_hidden() {
        // The fingerprint format is the engine's own — produce it by
        // running the same rules once, then freeze what came out.
        let policy = policy_with(None, Vec::new());
        let raw = validate_result(vec![RawFact::TsUnsafe {
            kind: "as_cross".to_string(),
            line: 64,
            reason: None,
        }]);
        let first = enrich_validate(&policy, "src/cells/greeting/index.ts", raw.clone());
        assert_eq!(first.conform_findings.len(), 1);
        assert!(!first.conform_findings[0].baselined);

        // freeze exactly what the engine fingerprinted
        let record = to_file_record("src/cells/greeting/index.ts", &raw);
        let facts = ts_extract_bridge::conform_facts(&record);
        let source = conform_core::SourceFacts {
            file: "src/cells/greeting/index.ts".to_string(),
            crate_name: String::new(),
            facts,
        };
        let owned = conform_cli_typescript::build_rules(&policy.config);
        let refs: Vec<&dyn conform_core::Rule> = owned.iter().map(|r| r.as_ref()).collect();
        let fps: Vec<String> = conform_core::check(&refs, &[source], None)
            .into_iter()
            .map(|f| f.fingerprint)
            .collect();
        let sanctioned = Policy {
            config: policy.config,
            baseline: fps,
        };
        let second = enrich_validate(&sanctioned, "src/cells/greeting/index.ts", raw);
        assert_eq!(second.conform_findings.len(), 1);
        assert!(second.conform_findings[0].baselined);
        assert!(
            second.advice.is_empty(),
            "sanctioned findings advise nothing"
        );
    }

    #[test]
    fn test_files_do_not_raise_domain_findings() {
        let policy = policy_with(None, Vec::new());
        let raw = validate_result(vec![RawFact::TsUnsafe {
            kind: "any_type".to_string(),
            line: 3,
            reason: None,
        }]);
        let enriched = enrich_validate(&policy, "src/cells/a/index.test.ts", raw);
        assert!(enriched.conform_findings.is_empty());
    }

    #[test]
    fn positions_parse_and_reject() {
        let p = parse_position("12:4").expect("ok");
        assert_eq!((p.line, p.character), (12, 4));
        assert!(parse_position("nope").is_err());
    }
}
