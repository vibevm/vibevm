//! rust-ai-native-tcg — the enrichment layer and op surface behind bin
//! `rust-ai-native-tcg` (TCG-PROTOCOL-RUST §2–§3): policy loading, the gate's
//! own rules run in-process over the gate's own fact extractor,
//! REQ-citing advice, and the scope semantics (module cells, newtype
//! brands). The serve relay and the bench harness live in their own
//! cells.

specmark::scope!("spec://rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#enrichment");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use conform_core::{Config, Fact, SourceFacts, baseline, baseline::Baseline, check};
use rust_ai_native_conform_frontend::RustFrontend;
use rust_ai_native_tcg_bridge::oracle::Completion;
use rust_ai_native_tcg_bridge::position::OuterPosition;
use rust_ai_native_tcg_bridge::{Diagnostic, RustOracle, ValidateOutcome};
use specmark::spec;

pub mod bench;
pub mod serve;

/// The ratchet baseline's conventional filename — the same file
/// `rust-ai-native init` writes and `rust-ai-native-conform check` reads
/// (pointer: rust-ai-native-cli's DEFAULT_CONFORM_BASELINE; the two
/// constants are held equal by the finding-parity test).
pub const DEFAULT_CONFORM_BASELINE: &str = "conform-baseline.json";

/// How long the relay waits for rust-analyzer quiescence at boot; a
/// pass degrades answers, it never fails the session (ORACLE-RUST §6).
pub const QUIESCENCE_BUDGET: std::time::Duration = std::time::Duration::from_secs(45);

/// The project policy the relay enriches through: the conform config,
/// the frozen ratchet baseline, and the root they belong to.
#[spec(implements = "spec://rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#enrichment")]
pub struct Policy {
    pub root: PathBuf,
    pub config: Config,
    pub baseline: Baseline,
}

impl Policy {
    /// Load `conform.toml` (or the announced topology default) and the
    /// baseline; the origin prints to stderr so a defaulted run can
    /// never masquerade as a configured one.
    pub fn load(root: &Path) -> Result<Self> {
        let (config, _origin) = rust_ai_native_conform::load_config_or_default(root)?;
        let baseline = baseline::load(&root.join(DEFAULT_CONFORM_BASELINE))?;
        Ok(Self {
            root: root.to_path_buf(),
            config,
            baseline,
        })
    }
}

/// One conform finding as the wire carries it (TCG-PROTOCOL-RUST §3):
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
/// the §3 fields. `markers` is reserved-empty in v0.1 (wire parity
/// with the TS shape; the specmark tag stream is the future filler).
#[derive(Debug, serde::Serialize)]
pub struct EnrichedValidate {
    pub diagnostics: Vec<Diagnostic>,
    pub facts: Vec<Fact>,
    pub markers: Vec<serde_json::Value>,
    pub conform_findings: Vec<WireFinding>,
    pub advice: Vec<String>,
    pub degraded: bool,
}

/// Derive `(crate_name, crate_ident, module)` for one repo-relative
/// file the way the engine's scanner does — a relay-local mirror of
/// conform-core's private mapping (store.rs `module_path`; exporting
/// it would mean a vendored-crate edit, and the finding-parity test
/// pins the two against each other instead).
///
/// ```
/// let (name, module) = rust_ai_native_tcg::derive_crate_module(
///     &["crates/*".to_string()],
///     "crates/rust-demo/src/cells/greeting.rs",
/// );
/// assert_eq!(name, "rust-demo");
/// assert_eq!(module, "rust_demo::cells::greeting");
/// ```
pub fn derive_crate_module(roots: &[String], file_rel: &str) -> (String, String) {
    let fwd = file_rel.replace('\\', "/");
    let mut crate_dir = String::new();
    for root in roots {
        let candidates: Vec<String> = if let Some(parent) = root.strip_suffix("/*") {
            // The glob form names each subdir; the file itself tells
            // us which one without listing the disk.
            let prefix = format!("{parent}/");
            fwd.strip_prefix(&prefix)
                .and_then(|rest| rest.split_once('/'))
                .map(|(first, _)| format!("{parent}/{first}"))
                .into_iter()
                .collect()
        } else {
            vec![root.trim_end_matches('/').to_string()]
        };
        for cand in candidates {
            let normal = cand.trim_start_matches("./").to_string();
            if (fwd.starts_with(&format!("{normal}/")) || normal == ".")
                && normal.len() >= crate_dir.len()
            {
                crate_dir = normal;
            }
        }
    }
    let crate_name = if crate_dir.is_empty() || crate_dir == "." {
        // Root-crate layout: the scanner derives the dir basename;
        // relay callers pass repo-relative paths, so fall back to the
        // last root component we can name.
        "crate".to_string()
    } else {
        crate_dir
            .rsplit('/')
            .next()
            .unwrap_or(crate_dir.as_str())
            .to_string()
    };
    let crate_ident = crate_name.replace('-', "_");
    let rel_in_crate = if crate_dir.is_empty() || crate_dir == "." {
        fwd.as_str()
    } else {
        fwd.strip_prefix(&format!("{crate_dir}/")).unwrap_or(&fwd)
    };
    let mut parts = vec![crate_ident.clone()];
    let trimmed = rel_in_crate.strip_prefix("src/").unwrap_or(rel_in_crate);
    let comps: Vec<&str> = trimmed.split('/').collect();
    for (i, comp) in comps.iter().enumerate() {
        let is_last = i + 1 == comps.len();
        if is_last {
            let stem = comp.strip_suffix(".rs").unwrap_or(comp);
            if !matches!(stem, "lib" | "main" | "mod") {
                parts.push(stem.to_string());
            }
        } else {
            parts.push((*comp).to_string());
        }
    }
    (crate_name, parts.join("::"))
}

/// Class-F advice for one finding rule: the GUIDE anchor and the move
/// that clears it (TCG-PROTOCOL-RUST §3).
fn advice_for(rule: &str) -> Option<String> {
    let text = match rule {
        "no-unwrap-in-domain" => {
            "an `unwrap`/`expect` in domain code — restructure so the type \
             carries the invariant, or record `#[spec(deviates, reason)]` \
             (GUIDE-AI-NATIVE-RUST §6)"
        }
        "seam-has-doctest" | "pub-doctest" => {
            "a public seam without a compiled doctest — add ONE canonical \
             example (GUIDE-AI-NATIVE-RUST §3 Class G)"
        }
        "error-enum-cites-req" | "error-message-cites-req" => {
            "an error message that cites no spec:// REQ — Class-F messages \
             carry the violated REQ and a fix surface \
             (GUIDE-AI-NATIVE-RUST §4)"
        }
        "ambient-env" => {
            "an ambient std::env read outside the sanctioned roots — declare \
             the read or route it through the composition root \
             (the R-001 rule)"
        }
        "file-length" => {
            "the file exceeds the position budget — split along the seam \
             into module-grain cells (GUIDE-AI-NATIVE-RUST §2)"
        }
        "unsafe-gate" => {
            "an unsafe block outside a designated audit crate — move it \
             there or record the deviation (GUIDE-AI-NATIVE-RUST §6)"
        }
        _ => return None,
    };
    Some(format!("{rule}: {text}"))
}

/// Enrich one validate outcome through the GATE'S OWN engine: the
/// frontend's facts over the effective text, the shared rule
/// assembly, findings flagged against the frozen baseline, advice
/// citing GUIDE REQs (TCG-PROTOCOL-RUST §3 — one engine, one truth).
#[spec(implements = "spec://rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#enrichment")]
pub fn enrich_validate(
    policy: &Policy,
    file_rel: &str,
    text: &str,
    outcome: ValidateOutcome,
) -> EnrichedValidate {
    use conform_core::Frontend;

    let (crate_name, module) = derive_crate_module(&policy.config.roots, file_rel);
    let frontend = RustFrontend;
    let facts = frontend.extract(file_rel, &crate_name, &module, text);
    let sf = SourceFacts {
        file: file_rel.replace('\\', "/"),
        crate_name,
        facts: facts.clone(),
    };
    let owned = rust_ai_native_conform::build_rules(&policy.config);
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
                && let Some(a) = advice_for(f.rule)
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
        facts,
        markers: Vec::new(),
        conform_findings,
        advice,
        degraded: outcome.degraded,
    }
}

/// A detected seam newtype — the Rust brand analog (D6): a pub tuple
/// struct with a single PRIVATE field, whose only constructor is a
/// parse fn. Honestly heuristic, and labelled so on the wire.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct BrandedNewtype {
    pub name: String,
    pub seam: String,
    pub heuristic: bool,
}

/// Syn-scan one file's text for the newtype-with-private-inner shape.
///
/// ```
/// let hits = rust_ai_native_tcg::detect_newtypes(
///     "pub struct GuestName(String);\npub struct Open(pub u32);\n",
///     "src/cells/greeting.rs",
/// );
/// assert_eq!(hits.len(), 1);
/// assert_eq!(hits[0].name, "GuestName");
/// assert!(hits[0].heuristic);
/// ```
#[spec(implements = "spec://rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#ops")]
pub fn detect_newtypes(text: &str, seam_file: &str) -> Vec<BrandedNewtype> {
    let Ok(ast) = syn::parse_file(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in &ast.items {
        if let syn::Item::Struct(s) = item
            && matches!(s.vis, syn::Visibility::Public(_))
            && let syn::Fields::Unnamed(fields) = &s.fields
            && fields.unnamed.len() == 1
            && matches!(fields.unnamed[0].vis, syn::Visibility::Inherited)
        {
            out.push(BrandedNewtype {
                name: s.ident.to_string(),
                seam: seam_file.replace('\\', "/"),
                heuristic: true,
            });
        }
    }
    out
}

/// The scope answer (TCG-PROTOCOL-RUST §2): symbols from a completion
/// sweep, the derived module cell, the enclosing seam file, and the
/// newtype brands.
#[derive(Debug, serde::Serialize)]
pub struct ScopeAnswer {
    pub symbols: Vec<serde_json::Value>,
    pub cell: String,
    pub seam_file: String,
    pub branded: Vec<BrandedNewtype>,
}

/// The seam file that registers this module: the enclosing `<dir>.rs`
/// (2018-style), `<dir>/mod.rs`, or the crate's `lib.rs` — the first
/// that exists.
pub fn seam_file_for(root: &Path, file_rel: &str) -> String {
    let fwd = file_rel.replace('\\', "/");
    if let Some((dir, _)) = fwd.rsplit_once('/') {
        let sibling = format!("{dir}.rs");
        if root.join(&sibling).is_file() {
            return sibling;
        }
        let mod_rs = format!("{dir}/mod.rs");
        if root.join(&mod_rs).is_file() {
            return mod_rs;
        }
        // Walk up toward the crate root looking for lib.rs.
        let mut cur = dir.to_string();
        loop {
            let lib = format!("{cur}/lib.rs");
            if root.join(&lib).is_file() {
                return lib;
            }
            match cur.rsplit_once('/') {
                Some((parent, _)) => cur = parent.to_string(),
                None => break,
            }
        }
    }
    fwd
}

/// Finalise the raw completion entries against the policy: the
/// prefix/max cut, then the `unsafe` flag on §6-banned continuations
/// (v0.1: `unwrap`/`expect` outside test files — name-grain, honest).
pub fn finalise_completions(
    entries: Vec<Completion>,
    file_rel: &str,
    prefix: Option<&str>,
    max: usize,
) -> Vec<serde_json::Value> {
    let in_test = file_rel.contains("/tests/") || file_rel.ends_with("tests.rs");
    entries
        .into_iter()
        .filter(|e| prefix.is_none_or(|p| e.name.starts_with(p)))
        .take(max)
        .map(|e| {
            let banned = !in_test && (e.name == "unwrap" || e.name == "expect");
            let mut v = serde_json::json!({
                "name": e.name,
                "kind": e.kind,
                "type_text": e.type_text,
                "unsafe": banned,
            });
            if banned {
                v["reason"] = serde_json::Value::String(
                    "would land a §6-banned form in domain code \
                     (GUIDE-AI-NATIVE-RUST §6: restructure or record \
                     #[spec(deviates)])"
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
) -> Result<RustOracle<rust_ai_native_tcg_bridge::client::ChildTransport>> {
    RustOracle::spawn(root, QUIESCENCE_BUDGET)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("spawning the rust-analyzer oracle")
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

/// The one-shot exit contract (mirrors typescript-ai-native-tcg): exit 1 on an
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
