use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};

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
                                &format!("cell module imports sibling cell module `{other_stem}`"),
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
/// crate root (`src/lib.rs`) of a gated crate is a seam, and so is
/// a public `trait` wherever it lives under `src/`; every seam
/// carries at least one compiled doctest of canonical use (card
/// scaffold-g-doctests). Re-exports and impl blocks are not item
/// facts, so the rule sees exactly the declared surface. (The
/// lib.rs-only scope was the original shape; audit 2026-06-12-08
/// recorded the submodule-trait gap — `GitBackend` and friends —
/// and the depth program widened the rule.)
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
            let is_lib_root = sf.file.ends_with("/src/lib.rs");
            if !sf.file.contains("/src/") {
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
                // lib.rs: every pub item is a seam; elsewhere only
                // pub traits are (a trait is a seam wherever it
                // lives — the 2026-06-12 widening).
                if !is_lib_root && kind != "trait" {
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

/// Class F (message grammar) — `error-message-cites-req`: a
/// thiserror variant's Display text in a gated crate itself
/// carries a `spec://` REQ URI, so a failing run is navigable
/// back to the requirement without source access. This is the
/// message half of Class F; `error-enum-cites-req` is the
/// attribute half (audit 2026-06-12-08 recorded the gap between
/// the two). Variants with no display template (`transparent`)
/// are out of scope.
///
/// ```
/// use conform_core::rules::ErrorMessageCitesReq;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let rule = ErrorMessageCitesReq { gated_crates: &["x"] };
/// let bare = SourceFacts {
///     file: "crates/x/src/error.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![Fact::ErrorVariant {
///         enum_symbol: "x::error::Error".into(),
///         variant: "Bad".into(),
///         message: "bad input".into(),
///         line: 4,
///         enum_attrs: vec!["spec(implements = \"spec://p/d#e\")".into()],
///     }],
/// };
/// assert_eq!(rule.check(&[bare]).len(), 1);
/// ```
pub struct ErrorMessageCitesReq {
    pub gated_crates: &'static [&'static str],
}

impl Rule for ErrorMessageCitesReq {
    fn id(&self) -> &'static str {
        "error-message-cites-req"
    }
    fn why(&self) -> &'static str {
        "errors are agent food: the Display text itself carries the REQ \
         URI, so a failing run is navigable back to the requirement \
         without source access (card scaffold-f-structured-diagnostics; \
         GUIDE-AI-NATIVE-RUST §4)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if !self.gated_crates.contains(&sf.crate_name.as_str()) {
                continue;
            }
            for f in &sf.facts {
                let Fact::ErrorVariant {
                    enum_symbol,
                    variant,
                    message,
                    line,
                    ..
                } = f
                else {
                    continue;
                };
                if message.is_empty() || message.contains("spec://") {
                    continue;
                }
                let name = enum_symbol.rsplit("::").next().unwrap_or(enum_symbol);
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: *line,
                    message: req_message(
                        "discipline://core/cards/scaffold-f-structured-diagnostics#ops",
                        &format!("`{name}::{variant}` display text cites no spec:// REQ"),
                        "embed the governing spec:// URI and a fix hint in the \
                         #[error(\"...\")] template",
                    ),
                    why: self.why(),
                    fingerprint: format!("error-message-cites-req|{}|{name}::{variant}", sf.file),
                });
            }
        }
        out.sort();
        out
    }
}

/// Guide §2 — `file-length`: a source file over the line budget
/// pages badly and buries invariants in its middle third; prefer
/// more, smaller, single-purpose files at equal token mass
/// (R3-003 "position is a resource"). The audit's god-file
/// inventory (2026-06-12-07) is this rule's frozen baseline; the
/// decomposition backlog shrinks it.
///
/// ```
/// use conform_core::rules::FileLength;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let rule = FileLength { max_lines: 600 };
/// let big = SourceFacts {
///     file: "crates/x/src/big.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![Fact::FileMetrics { lines: 1200 }],
/// };
/// assert_eq!(rule.check(&[big]).len(), 1);
/// ```
pub struct FileLength {
    pub max_lines: u32,
}

impl Rule for FileLength {
    fn id(&self) -> &'static str {
        "file-length"
    }
    fn why(&self) -> &'static str {
        "position is a resource: past the budget a file pages badly and \
         its middle third buries invariants — prefer more, smaller, \
         single-purpose files (GUIDE-AI-NATIVE-RUST §2, R3-003)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if !sf.file.contains("/src/") {
                continue;
            }
            for f in &sf.facts {
                let Fact::FileMetrics { lines } = f else {
                    continue;
                };
                if *lines <= self.max_lines {
                    continue;
                }
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: 1,
                    message: req_message(
                        "discipline://rust-ai-native/guide#surface-form",
                        &format!(
                            "{lines} lines exceeds the {}-line file budget",
                            self.max_lines
                        ),
                        "split along the file's responsibility seams into \
                         module-grain cells",
                    ),
                    why: self.why(),
                    fingerprint: format!("file-length|{}", sf.file),
                });
            }
        }
        out.sort();
        out
    }
}

/// Guide §6 — `no-unwrap-in-domain`: `.unwrap()` / `.expect()` in
/// domain logic converts a contract violation into a panic. Call
/// sites inside `#[cfg(test)]` modules and `#[test]` functions are
/// exempt (the facts carry `in_test`); legitimate boundaries
/// record `#[spec(deviates, reason)]` and freeze in the baseline.
///
/// ```
/// use conform_core::rules::NoUnwrapInDomain;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let rule = NoUnwrapInDomain { gated_crates: &["x"] };
/// let domain = SourceFacts {
///     file: "crates/x/src/m.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![
///         Fact::UnwrapUse { method: "unwrap".into(), line: 9, in_test: false },
///         Fact::UnwrapUse { method: "unwrap".into(), line: 90, in_test: true },
///     ],
/// };
/// assert_eq!(rule.check(&[domain]).len(), 1);
/// ```
pub struct NoUnwrapInDomain {
    pub gated_crates: &'static [&'static str],
}

impl Rule for NoUnwrapInDomain {
    fn id(&self) -> &'static str {
        "no-unwrap-in-domain"
    }
    fn why(&self) -> &'static str {
        "unwrap/expect in domain logic is a panic wearing a contract's \
         clothes: return through the layer's error enum, or mark the \
         justified boundary with #[spec(deviates, reason)] \
         (GUIDE-AI-NATIVE-RUST §6)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if !self.gated_crates.contains(&sf.crate_name.as_str()) {
                continue;
            }
            if !sf.file.contains("/src/") {
                continue;
            }
            // Per-file per-method ordinal fingerprints, never line
            // numbers (the stop.rs 33→35 lesson).
            let mut seen: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
            for f in &sf.facts {
                let Fact::UnwrapUse {
                    method,
                    line,
                    in_test,
                } = f
                else {
                    continue;
                };
                if *in_test {
                    continue;
                }
                let ordinal = seen.entry(method.as_str()).or_insert(0);
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: *line,
                    message: req_message(
                        "discipline://rust-ai-native/guide#bans-and-escape-hatches",
                        &format!("`.{method}()` in domain logic"),
                        "return through the layer's error enum, or record \
                         #[spec(deviates, reason)] at a justified boundary",
                    ),
                    why: self.why(),
                    fingerprint: format!("no-unwrap-in-domain|{}|{method}#{ordinal}", sf.file),
                });
                *seen.get_mut(method.as_str()).unwrap() += 1;
            }
        }
        out.sort();
        out
    }
}

/// Class D — `cell-has-oracle`: every `#[cell]`-manifested type
/// is referenced from at least one integration-test file of its
/// crate — the differential / characterization oracle the
/// replacement protocol requires (card scaffold-d, R-040). The
/// reference test is the static approximation of "an oracle
/// exists": a cell nobody's tests touch has no behavior net at
/// all, and replacing it merges blind.
///
/// ```
/// use conform_core::rules::CellHasOracle;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let cell = SourceFacts {
///     file: "crates/x/src/naive.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![Fact::Item {
///         kind: "struct".into(), symbol: "x::naive::Solver".into(), line: 3,
///         attrs: vec!["cell(seam = \"S\", variant = \"naive\")".into()],
///         is_pub: true, has_doctest: false,
///     }],
/// };
/// assert_eq!(CellHasOracle.check(&[cell]).len(), 1);
/// ```
pub struct CellHasOracle;

impl Rule for CellHasOracle {
    fn id(&self) -> &'static str {
        "cell-has-oracle"
    }
    fn why(&self) -> &'static str {
        "no replacement of a non-trivial cell merges without a differential \
         or characterization oracle against prior behavior (R-040; card \
         scaffold-d-differential-oracle) — a cell untouched by its crate's \
         tests has no behavior net at all"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for (type_name, file, crate_name) in cell_types(facts) {
            let tests_prefix = format!("crates/{crate_name}/tests/");
            let referenced = facts
                .iter()
                .filter(|sf| sf.file.starts_with(&tests_prefix))
                .any(|sf| {
                    sf.facts.iter().any(|f| match f {
                        Fact::Import { to_path, .. } => to_path.contains(&type_name),
                        Fact::Ctor { type_name: t, .. } => t == &type_name,
                        _ => false,
                    })
                });
            if referenced {
                continue;
            }
            let line = facts
                .iter()
                .find(|sf| sf.file == file)
                .and_then(|sf| {
                    sf.facts.iter().find_map(|f| match f {
                        Fact::Item { symbol, line, .. }
                            if symbol.ends_with(&format!("::{type_name}")) =>
                        {
                            Some(*line)
                        }
                        _ => None,
                    })
                })
                .unwrap_or(1);
            out.push(Finding {
                rule: self.id(),
                file: file.clone(),
                line,
                message: req_message(
                    "discipline://core/cards/scaffold-d-differential-oracle#ops",
                    &format!(
                        "cell `{type_name}` is referenced by no integration test \
                         in its crate — it has no behavior oracle"
                    ),
                    &format!(
                        "add a differential or characterization test under \
                         crates/{crate_name}/tests/ that drives `{type_name}`"
                    ),
                ),
                why: self.why(),
                fingerprint: format!("cell-has-oracle|{file}|{type_name}"),
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules;

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
    fn cell_has_oracle_satisfied_by_test_reference() {
        let cell = sf(
            "crates/x/src/naive.rs",
            "x",
            vec![cell_item("x::naive::NaiveSolver")],
        );
        let rule = rules::CellHasOracle;
        // No tests at all → finding.
        assert_eq!(rule.check(std::slice::from_ref(&cell)).len(), 1);
        // A test importing the cell type satisfies the rule.
        let test_import = sf(
            "crates/x/tests/oracle.rs",
            "x",
            vec![Fact::Import {
                from_module: "x::oracle".into(),
                to_path: "x::{DepSolver,NaiveSolver}".into(),
                line: 5,
            }],
        );
        assert!(rule.check(&[cell.clone(), test_import]).is_empty());
        // A test constructing the cell also satisfies it.
        let test_ctor = sf(
            "crates/x/tests/props.rs",
            "x",
            vec![Fact::Ctor {
                type_name: "NaiveSolver".into(),
                line: 9,
            }],
        );
        assert!(rule.check(&[cell, test_ctor]).is_empty());
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
}
