//! `conform-core` — the cross-language conformance engine core
//! (ENGINE-CONFORM v0.1; PLAYBOOK Phase 4).
//!
//! - [`Fact`] — the language-neutral normalized fact model (§3); the
//!   ledger's "facts class" instantiated.
//! - [`Store`] — content-addressed fact cache keyed by
//!   `(file content-hash, producer id+version)` (§3): facts never rot
//!   semantically; a 1-file diff re-extracts 1 file. Lives under
//!   `target/conform/` — derived data with a deterministic producer,
//!   never committed.
//! - [`Rule`] — rules as compiled queries over facts (§4); the
//!   declarative DSL is deliberately deferred (Open Question 2).
//! - [`sarif`] — byte-stable SARIF 2.1.0 rendering (§5): same inputs,
//!   byte-identical output, tested by double-run diff.
//! - [`baseline`] — the ratchet baseline (`conform-baseline.json`):
//!   pre-existing findings frozen per scope, new ones fail the gate,
//!   the file only shrinks.
//!
//! Frontier behaviour (B5, no cliffs): facts are extracted for the
//! whole workspace; **findings** are reported only inside the gate's
//! `--scope`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#facts");

/// One normalized fact (ENGINE-CONFORM §3). Variants carry exactly
/// what the checks consume; the schema grows with the rules (adding
/// a field or variant bumps the frontend version, which retires old
/// cache slots wholesale — facts never deserialize across schemas).
///
/// ```
/// use conform_core::Fact;
///
/// let fact = Fact::Item {
///     kind: "fn".into(),
///     symbol: "x::solve".into(),
///     line: 4,
///     attrs: vec![],
///     is_pub: true,
///     has_doctest: false,
/// };
/// assert!(matches!(fact, Fact::Item { is_pub: true, .. }));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "fact", rename_all = "snake_case")]
pub enum Fact {
    /// A declared item with its attributes (verbatim attribute text).
    Item {
        kind: String,
        symbol: String,
        line: u32,
        attrs: Vec<String>,
        /// `pub` at the item itself (visibility of the declaration,
        /// not reachability) — the seam signal for Class-G rules.
        #[serde(default)]
        is_pub: bool,
        /// The item's doc comment carries at least one fenced code
        /// block — a compiled doctest candidate (Class G).
        #[serde(default)]
        has_doctest: bool,
    },
    /// A `use` declaration: importing module → imported path.
    Import {
        from_module: String,
        to_path: String,
        line: u32,
    },
    /// A `<Type>::new(...)` construction site — the R-001 signal.
    Ctor { type_name: String, line: u32 },
    /// An `unsafe` block or `unsafe fn` body.
    UnsafeUse { context: String, line: u32 },
    /// A `#[error("...")]`-carrying enum variant (thiserror) with the
    /// enum's own attribute text — the Class-F diagnostics signal.
    ErrorVariant {
        enum_symbol: String,
        variant: String,
        message: String,
        line: u32,
        /// Attributes of the OWNING enum (where the REQ edge lives).
        enum_attrs: Vec<String>,
    },
}

/// Facts of one source file, with its repo-relative path.
///
/// ```
/// use conform_core::SourceFacts;
///
/// let sf = SourceFacts {
///     file: "crates/x/src/lib.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![],
/// };
/// assert_eq!(sf.crate_name, "x");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFacts {
    pub file: String,
    /// The crate directory name the file belongs to.
    pub crate_name: String,
    pub facts: Vec<Fact>,
}

/// A fact producer for one language (ENGINE-CONFORM §2). T-syn for
/// Phase 4; the trait carries id+version so the store key changes when
/// the frontend does.
///
/// The canonical implementation shape:
///
/// ```
/// use conform_core::{Fact, Frontend};
///
/// struct NullFrontend;
/// impl Frontend for NullFrontend {
///     fn id(&self) -> &'static str { "null" }
///     fn version(&self) -> &'static str { "1" }
///     fn extract(&self, _f: &str, _c: &str, _m: &str, _t: &str) -> Vec<Fact> {
///         Vec::new()
///     }
/// }
/// assert!(NullFrontend.extract("f.rs", "x", "x", "fn a() {}").is_empty());
/// ```
pub trait Frontend {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    /// Extract facts from one file. `module` is the module path the
    /// engine computed for it.
    fn extract(&self, file: &str, crate_name: &str, module: &str, text: &str) -> Vec<Fact>;
}

/// What one extraction run did — the producer log the incremental
/// acceptance test asserts on.
///
/// ```
/// let log = conform_core::ExtractionLog::default();
/// assert_eq!(log.cached, 0);
/// assert!(log.extracted.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct ExtractionLog {
    /// Files actually re-extracted this run (cache misses).
    pub extracted: Vec<String>,
    /// Cache hits.
    pub cached: usize,
}

/// Content-addressed fact store under `<repo>/target/conform/facts/`.
///
/// ```no_run
/// use conform_core::{ExtractionLog, Store};
/// # use conform_core::{Fact, Frontend};
/// # struct NullFrontend;
/// # impl Frontend for NullFrontend {
/// #     fn id(&self) -> &'static str { "null" }
/// #     fn version(&self) -> &'static str { "1" }
/// #     fn extract(&self, _f: &str, _c: &str, _m: &str, _t: &str) -> Vec<Fact> { Vec::new() }
/// # }
///
/// let repo = std::path::Path::new(".");
/// let store = Store::at_repo(repo);
/// let mut log = ExtractionLog::default();
/// let facts = store.extract_workspace(repo, &NullFrontend, &mut log).unwrap();
/// println!("{} file(s) extracted, {} cached", log.extracted.len(), log.cached);
/// # let _ = facts;
/// ```
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn at_repo(repo: &Path) -> Store {
        Store {
            root: repo.join("target").join("conform").join("facts"),
        }
    }

    fn slot(&self, frontend: &dyn Frontend, content_hash: &str) -> PathBuf {
        self.root
            .join(format!("{}-{}", frontend.id(), frontend.version()))
            .join(format!("{content_hash}.json"))
    }

    /// Extract facts for every workspace source file, reusing cached
    /// facts when `(content-hash, producer)` already has them.
    pub fn extract_workspace(
        &self,
        repo: &Path,
        frontend: &dyn Frontend,
        log: &mut ExtractionLog,
    ) -> Result<Vec<SourceFacts>> {
        let mut out = Vec::new();
        for (file, crate_name, module, path) in workspace_sources(repo) {
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let hash = content_hash(&text);
            let slot = self.slot(frontend, &hash);
            let facts: Vec<Fact> = if slot.exists() {
                log.cached += 1;
                let cached = std::fs::read_to_string(&slot)
                    .with_context(|| format!("reading {}", slot.display()))?;
                serde_json::from_str(&cached)
                    .with_context(|| format!("parsing {}", slot.display()))?
            } else {
                let fresh = frontend.extract(&file, &crate_name, &module, &text);
                if let Some(parent) = slot.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&slot, serde_json::to_string(&fresh)?)?;
                log.extracted.push(file.clone());
                fresh
            };
            out.push(SourceFacts {
                file,
                crate_name,
                facts,
            });
        }
        Ok(out)
    }
}

/// `sha256:<hex>` over LF-normalised text — the same convention the
/// rest of the project uses.
///
/// ```
/// let lf = conform_core::content_hash("a\nb\n");
/// let crlf = conform_core::content_hash("a\r\nb\r\n");
/// assert_eq!(lf, crlf);
/// assert!(lf.starts_with("sha256:"));
/// ```
pub fn content_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut hasher = Sha256::new();
    hasher.update(normalised.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(7 + digest.len() * 2);
    hex.push_str("sha256:");
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// Enumerate workspace sources: `(repo-rel file, crate name, module
/// path, absolute path)`. Same tree rscan walks: `crates/*/src` and
/// `xtask/src`, generated code excluded.
fn workspace_sources(repo: &Path) -> Vec<(String, String, String, PathBuf)> {
    let mut crate_dirs: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(repo.join("crates")) {
        for entry in rd.filter_map(Result::ok) {
            if entry.path().is_dir() {
                crate_dirs.push(entry.path());
            }
        }
    }
    crate_dirs.push(repo.join("xtask"));
    crate_dirs.sort();

    let mut out = Vec::new();
    for crate_dir in crate_dirs {
        let crate_name = crate_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let crate_ident = crate_name.replace('-', "_");
        let src = crate_dir.join("src");
        for entry in walkdir::WalkDir::new(&src)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().and_then(|e| e.to_str()) != Some("rs")
            {
                continue;
            }
            let rel_in_crate = path.strip_prefix(&crate_dir).unwrap_or(path);
            let rel_fwd = rel_in_crate.to_string_lossy().replace('\\', "/");
            if rel_fwd.contains("/generated/") {
                continue;
            }
            let module = module_path(&crate_ident, &rel_fwd);
            let file = path
                .strip_prefix(repo)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((file, crate_name.clone(), module, path.to_path_buf()));
        }
    }
    out
}

/// `src/lib.rs` → crate root; `src/foo.rs` → `crate::foo` — the rscan
/// scheme, duplicated here because the engine is specmap-independent
/// (the two reconcile when conform grows specmap-aware rules).
fn module_path(crate_ident: &str, rel_fwd: &str) -> String {
    let mut parts = vec![crate_ident.to_string()];
    let trimmed = rel_fwd.strip_prefix("src/").unwrap_or(rel_fwd);
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
    parts.join("::")
}

/// One finding with its A1 chain.
///
/// ```
/// use conform_core::Finding;
///
/// let f = Finding {
///     rule: "unsafe-gate",
///     file: "crates/x/src/lib.rs".into(),
///     line: 5,
///     message: conform_core::rules::req_message(
///         "discipline://rust-ai-native/guide#bans-and-escape-hatches",
///         "`unsafe` (block) outside a designated audit crate",
///         "move it or record the deviation",
///     ),
///     why: "unsafe is an audit boundary",
///     fingerprint: "unsafe-gate|crates/x/src/lib.rs|block#0".into(),
/// };
/// assert!(conform_core::rules::matches_req_grammar(&f.message));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub rule: &'static str,
    pub file: String,
    pub line: u32,
    pub message: String,
    /// Why the rule exists — the axiom trace rendered into SARIF.
    pub why: &'static str,
    /// Stable identity for the baseline: `rule|file|carrier`.
    pub fingerprint: String,
}

/// A rule is a compiled query over facts (ENGINE-CONFORM §4).
///
/// The canonical implementation shape — pure query in, findings out:
///
/// ```
/// use conform_core::{Finding, Rule, SourceFacts};
///
/// struct NoFindings;
/// impl Rule for NoFindings {
///     fn id(&self) -> &'static str { "no-findings" }
///     fn why(&self) -> &'static str { "demonstrates the query shape" }
///     fn check(&self, _facts: &[SourceFacts]) -> Vec<Finding> { Vec::new() }
/// }
/// assert!(NoFindings.check(&[]).is_empty());
/// ```
pub trait Rule {
    fn id(&self) -> &'static str;
    fn why(&self) -> &'static str;
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding>;
}

pub mod rules {
    use super::{Fact, Finding, Rule, SourceFacts};

    specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules");

    /// Render a finding message in the Class-F diagnostic grammar
    /// (card scaffold-f-structured-diagnostics, Band 3):
    /// `violates REQ <uri>: <why>; fix surface: <where>`. Every
    /// conform rule speaks this grammar — tool output is the agent's
    /// percept, and free text is wasted conditioning (R3-011).
    /// `discipline://` URIs cite the installed Discipline package
    /// (resolved against `vibevm.discipline.lock`); `spec://` URIs
    /// cite vibevm-hosted units. The convention is recorded in
    /// `spec/discipline/README.md`.
    ///
    /// ```
    /// let msg = conform_core::rules::req_message(
    ///     "spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules",
    ///     "what went wrong",
    ///     "where to fix it",
    /// );
    /// assert_eq!(
    ///     msg,
    ///     "violates REQ spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules: \
    ///      what went wrong; fix surface: where to fix it",
    /// );
    /// ```
    pub fn req_message(uri: &str, why: &str, fix_surface: &str) -> String {
        format!("violates REQ {uri}: {why}; fix surface: {fix_surface}")
    }

    /// The grammar acceptor the `diagnostic-cites-req` family checks
    /// against. Kept next to the renderer so they cannot drift.
    ///
    /// ```
    /// use conform_core::rules::{matches_req_grammar, req_message};
    ///
    /// assert!(matches_req_grammar(&req_message("spec://p/d#a", "why", "where")));
    /// assert!(!matches_req_grammar("Error: invalid configuration"));
    /// ```
    pub fn matches_req_grammar(message: &str) -> bool {
        let Some(rest) = message.strip_prefix("violates REQ ") else {
            return false;
        };
        let known_scheme = ["spec://", "discipline://", "misra://"]
            .iter()
            .any(|s| rest.starts_with(s));
        known_scheme && rest.contains(": ") && rest.contains("; fix surface: ")
    }

    /// The names of cell types, discovered from `#[cell(...)]`-carrying
    /// item facts, with the module (file) that declares each.
    fn cell_types(facts: &[SourceFacts]) -> Vec<(String, String, String)> {
        // (type name, declaring file, crate)
        let mut out = Vec::new();
        for sf in facts {
            for f in &sf.facts {
                if let Fact::Item { symbol, attrs, .. } = f
                    && attrs.iter().any(|a| a.starts_with("cell("))
                {
                    let type_name = symbol.rsplit("::").next().unwrap_or(symbol).to_string();
                    out.push((type_name, sf.file.clone(), sf.crate_name.clone()));
                }
            }
        }
        out.sort();
        out
    }

    /// R-001 — flag-sites: cell constructors appear only in the
    /// selection registry module.
    ///
    /// ```
    /// use conform_core::rules::FlagSites;
    /// use conform_core::Rule;
    ///
    /// let rule = FlagSites {
    ///     registry_file: "crates/app/src/registry.rs",
    ///     gated_crate: "app",
    /// };
    /// assert_eq!(rule.id(), "R-001");
    /// assert!(rule.check(&[]).is_empty());
    /// ```
    pub struct FlagSites {
        /// Repo-relative path of the one legal construction site.
        pub registry_file: &'static str,
        /// The crate whose construction sites are gated.
        pub gated_crate: &'static str,
    }

    impl Rule for FlagSites {
        fn id(&self) -> &'static str {
            "R-001"
        }
        fn why(&self) -> &'static str {
            "flag at the seam, never in the veins: the registry module is the \
             single place selection flags become cells (GUIDE-RUST §3)"
        }
        fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
            let cells: Vec<String> = cell_types(facts).into_iter().map(|(t, _, _)| t).collect();
            let mut out = Vec::new();
            for sf in facts {
                if sf.crate_name != self.gated_crate || sf.file == self.registry_file {
                    continue;
                }
                for f in &sf.facts {
                    if let Fact::Ctor { type_name, line } = f
                        && cells.contains(type_name)
                    {
                        out.push(Finding {
                            rule: self.id(),
                            file: sf.file.clone(),
                            line: *line,
                            message: req_message(
                                "discipline://rust-ai-native/guide#registry-and-flags",
                                &format!(
                                    "cell `{type_name}` constructed outside the selection registry"
                                ),
                                &format!(
                                    "construct cells only in {}; thread the instance in",
                                    self.registry_file
                                ),
                            ),
                            why: self.why(),
                            fingerprint: format!("R-001|{}|{type_name}", sf.file),
                        });
                    }
                }
            }
            out.sort();
            out
        }
    }

    /// R-002 — cell isolation: a cell module imports seams and core
    /// only, never a sibling cell.
    ///
    /// ```
    /// use conform_core::rules::CellIsolation;
    /// use conform_core::Rule;
    ///
    /// assert_eq!(CellIsolation.id(), "R-002");
    /// assert!(CellIsolation.check(&[]).is_empty());
    /// ```
    pub struct CellIsolation;

    impl Rule for CellIsolation {
        fn id(&self) -> &'static str {
            "R-002"
        }
        fn why(&self) -> &'static str {
            "a cell imports seams and core only — sibling-cell imports re-create \
             the tangle cells exist to prevent (GUIDE-RUST §1)"
        }
        fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
            let cells = cell_types(facts);
            let mut out = Vec::new();
            for sf in facts {
                // Only cell-declaring files are constrained.
                if !cells.iter().any(|(_, file, _)| file == &sf.file) {
                    continue;
                }
                for f in &sf.facts {
                    let Fact::Import { to_path, line, .. } = f else {
                        continue;
                    };
                    for (_t, other_file, other_crate) in &cells {
                        if other_file == &sf.file {
                            continue;
                        }
                        let other_stem = std::path::Path::new(other_file)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let other_ident = other_crate.replace('-', "_");
                        let same_crate = sf.crate_name == *other_crate
                            && (to_path.starts_with(&format!("crate::{other_stem}::"))
                                || to_path == &format!("crate::{other_stem}"));
                        let cross_crate = to_path
                            .starts_with(&format!("{other_ident}::{other_stem}::"))
                            || to_path == &format!("{other_ident}::{other_stem}");
                        if same_crate || cross_crate {
                            out.push(Finding {
                                rule: self.id(),
                                file: sf.file.clone(),
                                line: *line,
                                message: req_message(
                                    "discipline://rust-ai-native/guide#cells",
                                    &format!(
                                        "cell module imports sibling cell module `{other_stem}`"
                                    ),
                                    "import the seam trait or core types instead; route \
                                     cross-cell needs through the seam",
                                ),
                                why: self.why(),
                                fingerprint: format!("R-002|{}|{other_stem}", sf.file),
                            });
                        }
                    }
                }
            }
            out.sort();
            out.dedup();
            out
        }
    }

    /// unsafe-gate: `unsafe` appears only inside designated audit
    /// crates.
    ///
    /// ```
    /// use conform_core::rules::UnsafeGate;
    /// use conform_core::{Fact, Rule, SourceFacts};
    ///
    /// let rule = UnsafeGate { audit_crates: &["audited"] };
    /// let outside = SourceFacts {
    ///     file: "crates/a/src/lib.rs".into(),
    ///     crate_name: "a".into(),
    ///     facts: vec![Fact::UnsafeUse { context: "block".into(), line: 5 }],
    /// };
    /// let findings = rule.check(&[outside]);
    /// assert_eq!(findings.len(), 1);
    /// assert!(conform_core::rules::matches_req_grammar(&findings[0].message));
    /// ```
    pub struct UnsafeGate {
        pub audit_crates: &'static [&'static str],
    }

    impl Rule for UnsafeGate {
        fn id(&self) -> &'static str {
            "unsafe-gate"
        }
        fn why(&self) -> &'static str {
            "unsafe is an audit boundary: it lives in designated audit crates \
             or not at all (GUIDE-RUST §8, house rule)"
        }
        fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
            let mut out = Vec::new();
            for sf in facts {
                if self.audit_crates.contains(&sf.crate_name.as_str()) {
                    continue;
                }
                // Fingerprint by context + per-file ordinal, NOT by
                // line: a line-keyed fingerprint goes stale on any
                // edit above the block (the adopt-v0.3 Phase-0 lesson
                // — the stop.rs 33→35 shift), and a baseline that
                // rots on unrelated edits is a checker that lies.
                let mut seen: std::collections::BTreeMap<String, u32> =
                    std::collections::BTreeMap::new();
                for f in &sf.facts {
                    if let Fact::UnsafeUse { context, line } = f {
                        let ordinal = seen.entry(context.clone()).or_insert(0);
                        out.push(Finding {
                            rule: self.id(),
                            file: sf.file.clone(),
                            line: *line,
                            message: req_message(
                                "discipline://rust-ai-native/guide#bans-and-escape-hatches",
                                &format!("`unsafe` ({context}) outside a designated audit crate"),
                                "move the unsafe into an audit crate, or record the \
                                 deviation: #[spec(deviates, reason)] plus the wrapping \
                                 machinery",
                            ),
                            why: self.why(),
                            fingerprint: format!("unsafe-gate|{}|{context}#{ordinal}", sf.file),
                        });
                        *seen.get_mut(context).unwrap() += 1;
                    }
                }
            }
            out.sort();
            out
        }
    }

    /// Class G — `seam-has-doctest`: a public item declared at the
    /// crate root (`src/lib.rs`) of a gated crate is a seam; every
    /// seam carries at least one compiled doctest of canonical use
    /// (card scaffold-g-doctests). Re-exports and impl blocks are not
    /// item facts, so the rule sees exactly the declared surface.
    ///
    /// ```
    /// use conform_core::rules::SeamHasDoctest;
    /// use conform_core::{Fact, Rule, SourceFacts};
    ///
    /// let rule = SeamHasDoctest { gated_crates: &["x"] };
    /// let root = SourceFacts {
    ///     file: "crates/x/src/lib.rs".into(),
    ///     crate_name: "x".into(),
    ///     facts: vec![Fact::Item {
    ///         kind: "fn".into(), symbol: "x::solve".into(), line: 4,
    ///         attrs: vec![], is_pub: true, has_doctest: false,
    ///     }],
    /// };
    /// assert_eq!(rule.check(&[root]).len(), 1);
    /// ```
    pub struct SeamHasDoctest {
        pub gated_crates: &'static [&'static str],
    }

    impl Rule for SeamHasDoctest {
        fn id(&self) -> &'static str {
            "seam-has-doctest"
        }
        fn why(&self) -> &'static str {
            "the codebase is the few-shot prompt: a doctest that lies fails CI, \
             a prose example that lies ships — every public seam shows its one \
             canonical use as compiled code (card scaffold-g-doctests; R2C-008)"
        }
        fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
            let mut out = Vec::new();
            for sf in facts {
                if !self.gated_crates.contains(&sf.crate_name.as_str()) {
                    continue;
                }
                if !sf.file.ends_with("/src/lib.rs") {
                    continue;
                }
                for f in &sf.facts {
                    let Fact::Item {
                        kind,
                        symbol,
                        line,
                        is_pub,
                        has_doctest,
                        ..
                    } = f
                    else {
                        continue;
                    };
                    if !is_pub || *has_doctest {
                        continue;
                    }
                    let name = symbol.rsplit("::").next().unwrap_or(symbol);
                    out.push(Finding {
                        rule: self.id(),
                        file: sf.file.clone(),
                        line: *line,
                        message: req_message(
                            "discipline://core/cards/scaffold-g-doctests#ops",
                            &format!("public seam {kind} `{name}` has no compiled doctest"),
                            &format!(
                                "add one doctest on `{name}` showing the canonical \
                                 construction and use"
                            ),
                        ),
                        why: self.why(),
                        fingerprint: format!("seam-has-doctest|{}|{symbol}", sf.file),
                    });
                }
            }
            out.sort();
            out
        }
    }

    /// Class F — `error-enum-cites-req`: a thiserror enum in a gated
    /// crate carries a `#[spec(...)]` REQ edge (GUIDE §4: one error
    /// enum per layer, variants are contract surface). The edge makes
    /// the error layer navigable from the spec side; per-variant
    /// fix-surface hints are the cell sweep's per-message work.
    ///
    /// ```
    /// use conform_core::rules::ErrorEnumCitesReq;
    /// use conform_core::{Fact, Rule, SourceFacts};
    ///
    /// let rule = ErrorEnumCitesReq { gated_crates: &["x"] };
    /// let untagged = SourceFacts {
    ///     file: "crates/x/src/error.rs".into(),
    ///     crate_name: "x".into(),
    ///     facts: vec![Fact::ErrorVariant {
    ///         enum_symbol: "x::error::Error".into(),
    ///         variant: "Bad".into(),
    ///         message: "bad".into(),
    ///         line: 4,
    ///         enum_attrs: vec![],
    ///     }],
    /// };
    /// assert_eq!(rule.check(&[untagged]).len(), 1);
    /// ```
    pub struct ErrorEnumCitesReq {
        pub gated_crates: &'static [&'static str],
    }

    impl Rule for ErrorEnumCitesReq {
        fn id(&self) -> &'static str {
            "error-enum-cites-req"
        }
        fn why(&self) -> &'static str {
            "errors are contract surface and agent food: the error layer \
             carries a REQ edge so a failing run is navigable back to the \
             requirement it serves (card scaffold-f-structured-diagnostics; \
             GUIDE-AI-NATIVE-RUST §4)"
        }
        fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
            let mut out = Vec::new();
            let mut flagged: std::collections::BTreeSet<String> = Default::default();
            for sf in facts {
                if !self.gated_crates.contains(&sf.crate_name.as_str()) {
                    continue;
                }
                for f in &sf.facts {
                    let Fact::ErrorVariant {
                        enum_symbol,
                        line,
                        enum_attrs,
                        ..
                    } = f
                    else {
                        continue;
                    };
                    if enum_attrs.iter().any(|a| a.starts_with("spec(")) {
                        continue;
                    }
                    if !flagged.insert(enum_symbol.clone()) {
                        continue;
                    }
                    let name = enum_symbol.rsplit("::").next().unwrap_or(enum_symbol);
                    out.push(Finding {
                        rule: self.id(),
                        file: sf.file.clone(),
                        line: *line,
                        message: req_message(
                            "discipline://core/cards/scaffold-f-structured-diagnostics#ops",
                            &format!("thiserror enum `{name}` carries no #[spec] REQ edge"),
                            &format!(
                                "add #[spec(implements = \"spec://…\")] on `{name}` citing \
                                 the layer's error-contract unit"
                            ),
                        ),
                        why: self.why(),
                        fingerprint: format!("error-enum-cites-req|{}|{enum_symbol}", sf.file),
                    });
                }
            }
            out.sort();
            out
        }
    }
}

pub mod sarif {
    use super::{Finding, Rule};

    specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#determinism");

    /// Byte-stable minimal SARIF 2.1.0: stable ordering (findings are
    /// pre-sorted), no wall-clock, no absolute paths.
    ///
    /// ```
    /// use conform_core::rules::CellIsolation;
    /// use conform_core::sarif;
    ///
    /// let report = sarif::render(&[&CellIsolation], &[]);
    /// assert!(report.contains("\"version\": \"2.1.0\""));
    /// assert_eq!(report, sarif::render(&[&CellIsolation], &[]));
    /// ```
    pub fn render(rules: &[&dyn Rule], findings: &[Finding]) -> String {
        let rule_objs: Vec<serde_json::Value> = rules
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id(),
                    "shortDescription": { "text": r.why() }
                })
            })
            .collect();
        let results: Vec<serde_json::Value> = findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "ruleId": f.rule,
                    "level": "error",
                    "message": { "text": f.message },
                    "partialFingerprints": { "vibevmConform/v1": f.fingerprint },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": f.file },
                            "region": { "startLine": f.line }
                        }
                    }]
                })
            })
            .collect();
        let doc = serde_json::json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": { "driver": {
                    "name": "vibevm-conform",
                    "version": "0.1.0",
                    "rules": rule_objs
                }},
                "results": results
            }]
        });
        let mut s = serde_json::to_string_pretty(&doc).expect("sarif serialises");
        s.push('\n');
        s
    }
}

pub mod baseline {
    use std::path::Path;

    use anyhow::{Context, Result};
    use serde::{Deserialize, Serialize};

    use super::Finding;

    specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules");

    /// `conform-baseline.json`: frozen pre-existing findings, by
    /// fingerprint. The file only shrinks.
    ///
    /// ```
    /// use conform_core::baseline::Baseline;
    ///
    /// let frozen = Baseline {
    ///     schema: 1,
    ///     findings: vec!["unsafe-gate|crates/x/src/lib.rs|block#0".into()],
    /// };
    /// assert_eq!(frozen.findings.len(), 1);
    /// ```
    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct Baseline {
        pub schema: u32,
        #[serde(default)]
        pub findings: Vec<String>,
    }

    /// Load the baseline; an absent file is an empty baseline (the
    /// gate is then "no findings allowed at all").
    ///
    /// ```no_run
    /// let base = conform_core::baseline::load(
    ///     std::path::Path::new("conform-baseline.json"),
    /// ).unwrap();
    /// println!("{} frozen", base.findings.len());
    /// ```
    pub fn load(path: &Path) -> Result<Baseline> {
        if !path.exists() {
            return Ok(Baseline {
                schema: 1,
                findings: Vec::new(),
            });
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// Diff findings against the baseline: `(new, stale)` — new ones
    /// fail the gate; stale entries are prune candidates (the file may
    /// only shrink, so pruning is the legal direction).
    ///
    /// ```
    /// use conform_core::baseline::{Baseline, diff};
    ///
    /// let frozen = Baseline { schema: 1, findings: vec!["gone|x|0".into()] };
    /// let (new, stale) = diff(&frozen, &[]);
    /// assert!(new.is_empty());
    /// assert_eq!(stale, vec![&"gone|x|0".to_string()]);
    /// ```
    pub fn diff<'a>(
        baseline: &'a Baseline,
        findings: &'a [Finding],
    ) -> (Vec<&'a Finding>, Vec<&'a String>) {
        let new = findings
            .iter()
            .filter(|f| !baseline.findings.contains(&f.fingerprint))
            .collect();
        let stale = baseline
            .findings
            .iter()
            .filter(|fp| !findings.iter().any(|f| &f.fingerprint == *fp))
            .collect();
        (new, stale)
    }
}

/// Run every rule over the facts; report findings only inside `scope`
/// (a repo-relative path prefix; `None` = whole workspace). Facts are
/// already workspace-wide — the frontier rule (B5).
///
/// ```
/// use conform_core::rules::UnsafeGate;
/// use conform_core::{Fact, SourceFacts, check};
///
/// let gate = UnsafeGate { audit_crates: &[] };
/// let facts = vec![SourceFacts {
///     file: "crates/a/src/lib.rs".into(),
///     crate_name: "a".into(),
///     facts: vec![Fact::UnsafeUse { context: "block".into(), line: 5 }],
/// }];
/// assert_eq!(check(&[&gate], &facts, None).len(), 1);
/// assert!(check(&[&gate], &facts, Some("crates/b/")).is_empty());
/// ```
pub fn check(rules: &[&dyn Rule], facts: &[SourceFacts], scope: Option<&str>) -> Vec<Finding> {
    let mut findings: Vec<Finding> = rules.iter().flat_map(|r| r.check(facts)).collect();
    if let Some(prefix) = scope {
        findings.retain(|f| f.file.starts_with(prefix));
    }
    findings.sort();
    findings
}

/// Order facts deterministically inside one file's record so cache
/// files and SARIF stay byte-stable across runs.
///
/// ```
/// use conform_core::{SourceFacts, sort_source_facts};
///
/// let sf = |file: &str| SourceFacts {
///     file: file.into(), crate_name: "x".into(), facts: vec![],
/// };
/// let sorted = sort_source_facts(vec![sf("b.rs"), sf("a.rs")]);
/// assert_eq!(sorted[0].file, "a.rs");
/// ```
pub fn sort_source_facts(mut all: Vec<SourceFacts>) -> Vec<SourceFacts> {
    all.sort_by(|a, b| a.file.cmp(&b.file));
    all
}

/// Group findings per rule for the human one-liner.
///
/// ```
/// use conform_core::rules::UnsafeGate;
/// use conform_core::{Fact, Rule, SourceFacts, count_by_rule};
///
/// let gate = UnsafeGate { audit_crates: &[] };
/// let facts = vec![SourceFacts {
///     file: "crates/a/src/lib.rs".into(),
///     crate_name: "a".into(),
///     facts: vec![Fact::UnsafeUse { context: "block".into(), line: 5 }],
/// }];
/// let counts = count_by_rule(&gate.check(&facts));
/// assert_eq!(counts["unsafe-gate"], 1);
/// ```
pub fn count_by_rule(findings: &[Finding]) -> BTreeMap<&'static str, usize> {
    let mut map = BTreeMap::new();
    for f in findings {
        *map.entry(f.rule).or_insert(0) += 1;
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sf(file: &str, crate_name: &str, facts: Vec<Fact>) -> SourceFacts {
        SourceFacts {
            file: file.to_string(),
            crate_name: crate_name.to_string(),
            facts,
        }
    }

    fn cell_item(symbol: &str) -> Fact {
        Fact::Item {
            kind: "struct".into(),
            symbol: symbol.into(),
            line: 1,
            attrs: vec!["cell(seam = \"S\", variant = \"v\")".into()],
            is_pub: true,
            has_doctest: false,
        }
    }

    #[test]
    fn r001_flags_ctor_outside_registry() {
        let facts = vec![
            sf(
                "crates/vibe-resolver/src/naive.rs",
                "vibe-resolver",
                vec![cell_item("vibe_resolver::naive::NaiveDepSolver")],
            ),
            sf(
                "crates/vibe-cli/src/commands/install.rs",
                "vibe-cli",
                vec![Fact::Ctor {
                    type_name: "NaiveDepSolver".into(),
                    line: 7,
                }],
            ),
            sf(
                "crates/vibe-cli/src/registry.rs",
                "vibe-cli",
                vec![Fact::Ctor {
                    type_name: "NaiveDepSolver".into(),
                    line: 9,
                }],
            ),
        ];
        let rule = rules::FlagSites {
            registry_file: "crates/vibe-cli/src/registry.rs",
            gated_crate: "vibe-cli",
        };
        let found = rule.check(&facts);
        assert_eq!(found.len(), 1);
        assert!(found[0].file.ends_with("install.rs"));
    }

    #[test]
    fn r002_flags_sibling_cell_import() {
        let facts = vec![
            sf(
                "crates/x/src/alpha.rs",
                "x",
                vec![
                    cell_item("x::alpha::Alpha"),
                    Fact::Import {
                        from_module: "x::alpha".into(),
                        to_path: "crate::beta::Beta".into(),
                        line: 3,
                    },
                ],
            ),
            sf(
                "crates/x/src/beta.rs",
                "x",
                vec![cell_item("x::beta::Beta")],
            ),
        ];
        let found = rules::CellIsolation.check(&facts);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("beta"));
    }

    #[test]
    fn unsafe_gate_respects_audit_crates() {
        let facts = vec![
            sf(
                "crates/a/src/lib.rs",
                "a",
                vec![Fact::UnsafeUse {
                    context: "block".into(),
                    line: 5,
                }],
            ),
            sf(
                "crates/audited/src/lib.rs",
                "audited",
                vec![Fact::UnsafeUse {
                    context: "fn".into(),
                    line: 6,
                }],
            ),
        ];
        let rule = rules::UnsafeGate {
            audit_crates: &["audited"],
        };
        let found = rule.check(&facts);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file, "crates/a/src/lib.rs");
    }

    #[test]
    fn scope_filters_findings_not_facts() {
        let facts = vec![
            sf(
                "crates/a/src/lib.rs",
                "a",
                vec![Fact::UnsafeUse {
                    context: "block".into(),
                    line: 5,
                }],
            ),
            sf(
                "crates/b/src/lib.rs",
                "b",
                vec![Fact::UnsafeUse {
                    context: "block".into(),
                    line: 5,
                }],
            ),
        ];
        let gate = rules::UnsafeGate { audit_crates: &[] };
        let all = check(&[&gate], &facts, None);
        assert_eq!(all.len(), 2);
        let scoped = check(&[&gate], &facts, Some("crates/a/"));
        assert_eq!(scoped.len(), 1);
    }

    #[test]
    fn baseline_diff_news_and_stales() {
        let gate = rules::UnsafeGate { audit_crates: &[] };
        let facts = vec![sf(
            "crates/a/src/lib.rs",
            "a",
            vec![Fact::UnsafeUse {
                context: "block".into(),
                line: 5,
            }],
        )];
        let findings = check(&[&gate], &facts, None);
        let empty = baseline::Baseline {
            schema: 1,
            findings: vec![],
        };
        let (new, stale) = baseline::diff(&empty, &findings);
        assert_eq!(new.len(), 1);
        assert!(stale.is_empty());

        let frozen = baseline::Baseline {
            schema: 1,
            findings: vec![findings[0].fingerprint.clone(), "gone|x|1".into()],
        };
        let (new, stale) = baseline::diff(&frozen, &findings);
        assert!(new.is_empty());
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn req_grammar_renderer_and_acceptor_agree() {
        let msg = rules::req_message(
            "discipline://core/cards/scaffold-g-doctests#ops",
            "public seam fn `solve` has no compiled doctest",
            "add one doctest on `solve`",
        );
        assert!(rules::matches_req_grammar(&msg), "{msg}");
        assert!(!rules::matches_req_grammar("free text error"));
        assert!(!rules::matches_req_grammar(
            "violates REQ http://nope: x; fix surface: y"
        ));
        assert!(!rules::matches_req_grammar(
            "violates REQ spec://p/d#a: missing the fix surface"
        ));
    }

    #[test]
    fn every_rule_message_speaks_the_req_grammar() {
        // Class F applied to conform itself: each rule's findings on a
        // synthetic violating corpus must match the grammar.
        let corpus = vec![
            sf(
                "crates/x/src/alpha.rs",
                "x",
                vec![
                    cell_item("x::alpha::Alpha"),
                    Fact::Import {
                        from_module: "x::alpha".into(),
                        to_path: "crate::beta::Beta".into(),
                        line: 3,
                    },
                ],
            ),
            sf(
                "crates/x/src/beta.rs",
                "x",
                vec![
                    cell_item("x::beta::Beta"),
                    Fact::Ctor {
                        type_name: "Alpha".into(),
                        line: 9,
                    },
                    Fact::UnsafeUse {
                        context: "block".into(),
                        line: 12,
                    },
                ],
            ),
            sf(
                "crates/x/src/lib.rs",
                "x",
                vec![
                    Fact::Item {
                        kind: "fn".into(),
                        symbol: "x::solve".into(),
                        line: 4,
                        attrs: vec![],
                        is_pub: true,
                        has_doctest: false,
                    },
                    Fact::ErrorVariant {
                        enum_symbol: "x::Error".into(),
                        variant: "Bad".into(),
                        message: "bad thing".into(),
                        line: 8,
                        enum_attrs: vec![],
                    },
                ],
            ),
        ];
        let flag_sites = rules::FlagSites {
            registry_file: "crates/x/src/registry.rs",
            gated_crate: "x",
        };
        let isolation = rules::CellIsolation;
        let unsafe_gate = rules::UnsafeGate { audit_crates: &[] };
        let doctests = rules::SeamHasDoctest {
            gated_crates: &["x"],
        };
        let err_req = rules::ErrorEnumCitesReq {
            gated_crates: &["x"],
        };
        let all: Vec<&dyn Rule> = vec![&flag_sites, &isolation, &unsafe_gate, &doctests, &err_req];
        for rule in &all {
            let found = rule.check(&corpus);
            assert!(!found.is_empty(), "rule {} found nothing", rule.id());
            for f in found {
                assert!(
                    rules::matches_req_grammar(&f.message),
                    "rule {} message off-grammar: {}",
                    rule.id(),
                    f.message
                );
            }
        }
    }

    #[test]
    fn seam_has_doctest_gates_pub_root_items_only() {
        let facts = vec![sf(
            "crates/x/src/lib.rs",
            "x",
            vec![
                Fact::Item {
                    kind: "fn".into(),
                    symbol: "x::documented".into(),
                    line: 1,
                    attrs: vec![],
                    is_pub: true,
                    has_doctest: true,
                },
                Fact::Item {
                    kind: "fn".into(),
                    symbol: "x::bare".into(),
                    line: 5,
                    attrs: vec![],
                    is_pub: true,
                    has_doctest: false,
                },
                Fact::Item {
                    kind: "fn".into(),
                    symbol: "x::private".into(),
                    line: 9,
                    attrs: vec![],
                    is_pub: false,
                    has_doctest: false,
                },
            ],
        )];
        let rule = rules::SeamHasDoctest {
            gated_crates: &["x"],
        };
        let found = rule.check(&facts);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("`bare`"));
        // Non-root files are not seams for this rule.
        let nested = vec![sf(
            "crates/x/src/inner.rs",
            "x",
            vec![Fact::Item {
                kind: "fn".into(),
                symbol: "x::inner::bare".into(),
                line: 5,
                attrs: vec![],
                is_pub: true,
                has_doctest: false,
            }],
        )];
        assert!(rule.check(&nested).is_empty());
    }

    #[test]
    fn error_enum_cites_req_flags_once_per_enum() {
        let facts = vec![sf(
            "crates/x/src/error.rs",
            "x",
            vec![
                Fact::ErrorVariant {
                    enum_symbol: "x::error::Error".into(),
                    variant: "A".into(),
                    message: "a".into(),
                    line: 4,
                    enum_attrs: vec![],
                },
                Fact::ErrorVariant {
                    enum_symbol: "x::error::Error".into(),
                    variant: "B".into(),
                    message: "b".into(),
                    line: 6,
                    enum_attrs: vec![],
                },
                Fact::ErrorVariant {
                    enum_symbol: "x::error::Tagged".into(),
                    variant: "C".into(),
                    message: "c".into(),
                    line: 14,
                    enum_attrs: vec!["spec(implements = \"spec://p/d#err\")".into()],
                },
            ],
        )];
        let rule = rules::ErrorEnumCitesReq {
            gated_crates: &["x"],
        };
        let found = rule.check(&facts);
        assert_eq!(found.len(), 1, "one finding per untagged enum: {found:?}");
        assert!(found[0].message.contains("`Error`"));
    }

    #[test]
    fn unsafe_gate_fingerprint_survives_line_shifts() {
        let before = vec![sf(
            "crates/a/src/lib.rs",
            "a",
            vec![Fact::UnsafeUse {
                context: "block".into(),
                line: 33,
            }],
        )];
        let after = vec![sf(
            "crates/a/src/lib.rs",
            "a",
            vec![Fact::UnsafeUse {
                context: "block".into(),
                line: 35,
            }],
        )];
        let rule = rules::UnsafeGate { audit_crates: &[] };
        let fp_before = rule.check(&before)[0].fingerprint.clone();
        let fp_after = rule.check(&after)[0].fingerprint.clone();
        assert_eq!(
            fp_before, fp_after,
            "a pure line shift must not change the fingerprint"
        );
    }

    #[test]
    fn sarif_is_byte_stable() {
        let gate = rules::UnsafeGate { audit_crates: &[] };
        let facts = vec![sf(
            "crates/a/src/lib.rs",
            "a",
            vec![Fact::UnsafeUse {
                context: "block".into(),
                line: 5,
            }],
        )];
        let findings = check(&[&gate], &facts, None);
        let a = sarif::render(&[&gate], &findings);
        let b = sarif::render(&[&gate], &findings);
        assert_eq!(a, b);
        assert!(a.contains("\"ruleId\": \"unsafe-gate\""));
    }
}
