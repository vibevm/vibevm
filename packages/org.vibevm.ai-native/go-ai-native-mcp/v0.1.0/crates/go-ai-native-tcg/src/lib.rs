//! go-ai-native-tcg — the enrichment layer and op surface behind bin
//! `go-ai-native-tcg` (TCG-PROTOCOL-GO §2–§3): policy loading, the
//! gate's own rules run in-process over the gate's own fact extractor
//! (go-extract, the overlay form), REQ-citing advice, the FILLED
//! markers field (the delta the protocol names against the Rust
//! relay), and the scope semantics (package cells, defined-type
//! brands). The serve relay and the bench harness live in their own
//! cells.

specmark::scope!("spec://go-ai-native-lang/go/mechanisms/TCG-PROTOCOL-GO-v0.1#enrichment");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use conform_core::{Config, Fact, SourceFacts, baseline, baseline::Baseline, check};
use go_ai_native_extract_bridge::{RawFact, RawMarker};
use go_ai_native_tcg_bridge::oracle::Completion;
use go_ai_native_tcg_bridge::position::OuterPosition;
use go_ai_native_tcg_bridge::{Diagnostic, GoOracle, ValidateOutcome};
use specmark::spec;

pub mod bench;
pub mod serve;

/// The ratchet baseline's conventional filename — the same file
/// `go-ai-native init` writes and `go-ai-native-conform check` reads
/// (the finding-parity test holds the constants equal).
pub const DEFAULT_GO_BASELINE: &str = "go-ai-native-conform-baseline.json";

/// How long the relay waits for gopls readiness at boot; a pass
/// degrades answers, it never fails the session (ORACLE-GO §6).
pub const READINESS_BUDGET: std::time::Duration = std::time::Duration::from_secs(45);

/// The project policy the relay enriches through: the conform config,
/// the frozen ratchet baseline, the materialised extractor, and the
/// root they belong to.
#[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-PROTOCOL-GO-v0.1#enrichment")]
pub struct Policy {
    pub root: PathBuf,
    pub config: Config,
    pub baseline: Baseline,
    pub extractor: PathBuf,
}

impl Policy {
    /// Load `conform.toml` (or the announced topology default), the
    /// baseline, and materialise the extractor; the origin prints to
    /// stderr so a defaulted run can never masquerade as configured.
    pub fn load(root: &Path) -> Result<Self> {
        let (config, origin) = Config::load_or_default(root)?;
        eprintln!(
            "go-ai-native-tcg: policy conform.toml ({}).",
            match origin {
                conform_core::ConfigOrigin::Loaded => "loaded",
                conform_core::ConfigOrigin::Defaulted => "DEFAULTED — run `go-ai-native init`",
            }
        );
        let baseline = baseline::load(&root.join(DEFAULT_GO_BASELINE))?;
        let extractor = go_ai_native_extract_bridge::materialise_extractor(root)
            .context("materialising go-extract")?;
        Ok(Self {
            root: root.to_path_buf(),
            config,
            baseline,
            extractor,
        })
    }
}

/// One conform finding as the wire carries it (TCG-PROTOCOL-GO §3):
/// flagged against the frozen ratchet so the agent sees sanctioned
/// findings distinctly.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WireFinding {
    pub rule: String,
    pub message: String,
    pub line: u32,
    pub baselined: bool,
}

/// The enriched validate result — the §2 oracle answer widened with
/// the §3 fields. `markers` is FILLED (the //spec: directive stream) —
/// the per-language capability delta the protocol names against the
/// Rust relay's reserved-empty field.
#[derive(Debug, serde::Serialize)]
pub struct EnrichedValidate {
    pub diagnostics: Vec<Diagnostic>,
    pub facts: Vec<RawFact>,
    pub markers: Vec<RawMarker>,
    pub conform_findings: Vec<WireFinding>,
    pub advice: Vec<String>,
    pub degraded: bool,
}

/// Class-F advice for one finding rule: the GUIDE anchor and the move
/// that clears it (TCG-PROTOCOL-GO §3).
fn advice_for(rule: &str, kind_hint: &str) -> Option<String> {
    let text = match rule {
        "go-unsafe-in-domain" => match kind_hint {
            "init_decl" | "blank_import" => {
                "import-time registration in a cell — move it to the composition \
                 root or a boundary adapter (GUIDE-AI-NATIVE-GO §2)"
            }
            "ambient_call" => {
                "an ambient default in a cell — inject the capability as a \
                 private narrow interface (GUIDE-AI-NATIVE-GO §2)"
            }
            "naked_go" => {
                "an unowned goroutine — own it with errgroup/WaitGroup + context \
                 (GUIDE-AI-NATIVE-GO §5)"
            }
            "error_string_match" => {
                "matching an error's prose — consume the seam's closed error set \
                 via errors.As on its Code (GUIDE-AI-NATIVE-GO §5)"
            }
            "t_skip" => {
                "a skipped known-failing test — record it in \
                 discipline/registry/tests-baseline.json instead \
                 (GUIDE-AI-NATIVE-GO §10)"
            }
            "seam_error_missing_req" => {
                "a seam error type without a Spec field — carry the violated \
                 spec:// URI (GUIDE-AI-NATIVE-GO §5)"
            }
            _ => {
                "a ban-census site — restructure so the type or the composition \
                 root carries it, or record //spec:deviates with a reason \
                 (GUIDE-AI-NATIVE-GO §7)"
            }
        },
        "go-cell-isolation" => {
            "a sibling-cell import — depend on the seams package instead; only \
             the registry imports cells (GUIDE-AI-NATIVE-GO §2)"
        }
        "file-length" => {
            "the file exceeds the position budget — move a cohesive slice into a \
             sibling file of the same package (GUIDE-AI-NATIVE-GO §15)"
        }
        _ => return None,
    };
    Some(format!("{rule}: {text}"))
}

/// The census kind a finding's fingerprint carries (`rule|file|kind#line`).
fn kind_of_fingerprint(fp: &str) -> &str {
    fp.rsplit('|')
        .next()
        .and_then(|tail| tail.split('#').next())
        .unwrap_or("")
}

/// Enrich one validate outcome through the GATE'S OWN engine: the
/// extractor's facts+markers over the effective text (the overlay
/// form), the shared rule assembly, findings flagged against the
/// frozen baseline, advice citing GUIDE REQs (TCG-PROTOCOL-GO §3 —
/// one engine, one truth).
#[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-PROTOCOL-GO-v0.1#enrichment")]
pub fn enrich_validate(
    policy: &Policy,
    file_rel: &str,
    text: &str,
    outcome: ValidateOutcome,
) -> EnrichedValidate {
    let record = match go_ai_native_extract_bridge::extract_content(
        &policy.root,
        &policy.extractor,
        file_rel,
        text,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("go-ai-native-tcg: enrichment extraction failed — {e}");
            return EnrichedValidate {
                diagnostics: outcome.diagnostics,
                facts: Vec::new(),
                markers: Vec::new(),
                conform_findings: Vec::new(),
                advice: Vec::new(),
                degraded: true,
            };
        }
    };
    let sf = SourceFacts {
        file: file_rel.replace('\\', "/"),
        crate_name: "go".to_string(),
        facts: go_ai_native_extract_bridge::conform_facts(&record),
    };
    let owned = go_ai_native_conform::build_rules(&policy.config);
    let rule_refs: Vec<&dyn conform_core::Rule> = owned.iter().map(|r| r.as_ref()).collect();
    let findings = check(&rule_refs, &[sf], None);
    let mut advice: Vec<String> = Vec::new();
    let conform_findings: Vec<WireFinding> = findings
        .iter()
        .map(|f| {
            let baselined = policy
                .baseline
                .findings
                .iter()
                .any(|fp| fp == &f.fingerprint);
            if !baselined
                && let Some(a) = advice_for(f.rule, kind_of_fingerprint(&f.fingerprint))
                && !advice.contains(&a)
            {
                advice.push(a);
            }
            WireFinding {
                rule: f.rule.to_string(),
                message: f.message.clone(),
                line: f.line,
                baselined,
            }
        })
        .collect();
    EnrichedValidate {
        diagnostics: outcome.diagnostics,
        facts: record.facts,
        markers: record.markers,
        conform_findings,
        advice,
        degraded: outcome.degraded,
    }
}

/// A detected seam brand — the Go analog (TCG-PROTOCOL-GO §2): an
/// exported DEFINED TYPE over a primitive declared in a seam file.
/// go-extract-detected; honestly heuristic, and labelled so.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct BrandedType {
    pub name: String,
    pub seam: String,
    pub heuristic: bool,
}

/// Brands out of one extraction record: type items carrying a
/// primitive `underlying`.
pub fn brands_of(record: &go_ai_native_extract_bridge::FileRecord) -> Vec<BrandedType> {
    record
        .facts
        .iter()
        .filter_map(|f| match f {
            RawFact::Item {
                kind,
                symbol,
                is_exported,
                underlying: Some(_),
                ..
            } if kind == "type" && *is_exported => Some(BrandedType {
                name: symbol.clone(),
                seam: record.file.clone(),
                heuristic: true,
            }),
            _ => None,
        })
        .collect()
}

/// The scope answer (TCG-PROTOCOL-GO §2): symbols from a completion
/// sweep, the derived package cell, the seams package, and the
/// defined-type brands.
#[derive(Debug, serde::Serialize)]
pub struct ScopeAnswer {
    pub symbols: Vec<serde_json::Value>,
    pub cell: String,
    pub seam_file: String,
    pub branded: Vec<BrandedType>,
}

/// The cell a file belongs to: its path under the policy's
/// `cells_dir` (`internal/cells/plan/plan.go` → `plan`), else its
/// package directory.
///
/// ```
/// let cfg = conform_core::Config::default();
/// // no cells_dir configured → the package directory stands in
/// assert_eq!(
///     go_ai_native_tcg::cell_of(&cfg, "internal/sim/world.go"),
///     "internal/sim",
/// );
/// ```
pub fn cell_of(config: &Config, file_rel: &str) -> String {
    let fwd = file_rel.replace('\\', "/");
    if let Some(cells_dir) = &config.go.cells_dir {
        let prefix = format!("{}/", cells_dir.trim_matches('/'));
        if let Some(rest) = fwd.strip_prefix(&prefix)
            && let Some(cell) = rest.split('/').next()
            && !cell.is_empty()
        {
            return cell.to_string();
        }
    }
    fwd.rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or(fwd)
}

/// The seams package directory the policy names (for `scope`'s
/// `seam_file`), else the file's own directory.
pub fn seam_file_for(config: &Config, file_rel: &str) -> String {
    if let Some(seams) = &config.go.seams_pkg {
        return seams.trim_matches('/').to_string();
    }
    let fwd = file_rel.replace('\\', "/");
    fwd.rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or(fwd)
}

/// Ambient-default identifiers whose insertion in a cell would land a
/// §2-banned form (the completion `unsafe` heuristic — name-grain,
/// honestly labelled in the brief).
const AMBIENT_COMPLETIONS: &[&str] = &[
    "Getenv",
    "Setenv",
    "LookupEnv",
    "Now",
    "Since",
    "Until",
    "DefaultClient",
    "DefaultServeMux",
    "DefaultTransport",
];

/// Finalise the raw completion entries against the policy: the
/// prefix/max cut, then the `unsafe` flag on §2-banned continuations
/// inside cell files.
pub fn finalise_completions(
    config: &Config,
    entries: Vec<Completion>,
    file_rel: &str,
    prefix: Option<&str>,
    max: usize,
) -> Vec<serde_json::Value> {
    let in_cell = config
        .go
        .cells_dir
        .as_deref()
        .is_some_and(|d| file_rel.replace('\\', "/").starts_with(&format!("{d}/")));
    let in_test = file_rel.ends_with("_test.go");
    entries
        .into_iter()
        .filter(|e| prefix.is_none_or(|p| e.name.starts_with(p)))
        .take(max)
        .map(|e| {
            let banned = in_cell && !in_test && AMBIENT_COMPLETIONS.contains(&e.name.as_str());
            let mut v = serde_json::json!({
                "name": e.name,
                "kind": e.kind,
                "type_text": e.type_text,
                "unsafe": banned,
            });
            if banned {
                v["reason"] = serde_json::Value::String(
                    "would land an ambient default in a cell \
                     (GUIDE-AI-NATIVE-GO §2: inject the capability instead)"
                        .to_string(),
                );
            }
            v
        })
        .collect()
}

/// Spawn the oracle for a root with the standing budgets — the shared
/// entry the serve relay and every one-shot form use.
pub fn spawn_oracle(
    root: &Path,
) -> Result<GoOracle<go_ai_native_tcg_bridge::client::ChildTransport>> {
    GoOracle::spawn(root, READINESS_BUDGET)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("spawning the gopls oracle")
}

/// Read `--content-from` (a path, or `-` for stdin).
pub fn read_content_from(spec: &str) -> Result<String> {
    if spec == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading hypothetical content from stdin")?;
        Ok(buf)
    } else {
        std::fs::read_to_string(spec).with_context(|| format!("reading {spec}"))
    }
}

/// The one-shot exit contract (sibling parity): exit 1 on an
/// error-grade diagnostic OR a non-baselined finding.
pub fn validate_exit_code(enriched: &EnrichedValidate) -> i32 {
    let has_error = enriched.diagnostics.iter().any(|d| d.category == "error");
    let has_new_finding = enriched.conform_findings.iter().any(|f| !f.baselined);
    i32::from(has_error || has_new_finding)
}

/// Positions parse as `L:C` (1-based line, 0-based character — the
/// outer convention).
pub fn parse_position(s: &str) -> Result<OuterPosition> {
    let (l, c) = s
        .split_once(':')
        .with_context(|| format!("position `{s}` is not L:C"))?;
    Ok(OuterPosition {
        line: l.parse().with_context(|| format!("line in `{s}`"))?,
        character: c.parse().with_context(|| format!("character in `{s}`"))?,
    })
}

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
