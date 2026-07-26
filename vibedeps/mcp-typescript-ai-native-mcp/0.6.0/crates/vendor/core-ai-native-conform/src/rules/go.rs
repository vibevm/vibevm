//! The Go rule family (GUIDE-AI-NATIVE-GO §2, §5, §7), fed by the
//! `go-extract` frontend's facts: the ban census with its
//! recorded-deviation escape hatch, and cell isolation over import
//! paths. Defined ONCE here — the neutral engine — so the rule cannot
//! drift between language projections (the same consolidation argument
//! that homes the TypeScript family here).

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};
use crate::rules::req_message;

const GO_GUIDE_CELLS: &str = "discipline://go-ai-native-lang/guide#cells";
const GO_GUIDE_ERRORS: &str = "discipline://go-ai-native-lang/guide#errors";
const GO_GUIDE_BANS: &str = "discipline://go-ai-native-lang/guide#bans";
const GO_GUIDE_REPLACEMENT: &str = "discipline://go-ai-native-lang/guide#replacement";

/// `go-unsafe-in-domain` — the Go ban census as Class-F findings:
/// `init()` declarations, blank imports, ambient defaults
/// (`os.Getenv`, `time.Now`, `http.DefaultClient`-class), and naked
/// `go` statements are banned INSIDE CELLS (§2, §5 — the composition
/// root and boundary adapters are their sanctioned homes, so these
/// kinds fire only under `cells_dir`); error-string matching (§5),
/// reasonless suppression directives (§1), a seam error type without
/// its REQ citation (§5), and `t.Skip` on tests (§10 — the registry is
/// the only xfail home) fire everywhere. A site covered by a reasoned
/// `//spec:deviates … reason="…"` is recorded testimony and is
/// honoured, not flagged. Value-level bans skip `_test.go` files
/// (capability injection is not demanded of fixtures); `t_skip` fires
/// ONLY there.
///
/// ```
/// use core_ai_native_conform::rules::GoUnsafeInDomain;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let rule = GoUnsafeInDomain::new(Some("internal/cells"));
/// let facts = vec![SourceFacts {
///     file: "internal/cells/plan/plan.go".into(),
///     crate_name: "demo".into(),
///     facts: vec![Fact::GoUnsafe {
///         kind: "init_decl".into(),
///         line: 7,
///         in_test: false,
///         reason: None,
///     }],
/// }];
/// let findings = rule.check(&facts);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].message.contains("init()"));
/// ```
pub struct GoUnsafeInDomain {
    cells_prefix: Option<String>,
}

impl GoUnsafeInDomain {
    pub fn new(cells_dir: Option<&str>) -> GoUnsafeInDomain {
        GoUnsafeInDomain {
            cells_prefix: cells_dir.map(|d| format!("{}/", d.trim_matches('/'))),
        }
    }

    fn in_cells(&self, file: &str) -> bool {
        self.cells_prefix
            .as_deref()
            .is_some_and(|p| file.starts_with(p))
    }
}

impl Rule for GoUnsafeInDomain {
    fn id(&self) -> &'static str {
        "go-unsafe-in-domain"
    }
    fn why(&self) -> &'static str {
        "Go's prescriptions stop one step short of contract: import-time registration, \
         ambient defaults, unowned goroutines, and skipped tests are exactly where a \
         cell's closure breaks silently"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for source in facts {
            let in_cells = self.in_cells(&source.file);
            for fact in &source.facts {
                let Fact::GoUnsafe {
                    kind,
                    line,
                    in_test,
                    reason,
                } = fact
                else {
                    continue;
                };
                // A reasoned deviation covering the site is testimony,
                // not a finding — except suppressions, whose reason is
                // exactly what the census checks.
                if reason.is_some() && kind != "reasonless_suppression" {
                    continue;
                }
                // Cell-scoped kinds fire only under cells_dir; the
                // composition root and boundary adapters are their
                // sanctioned homes.
                let cell_scoped = matches!(
                    kind.as_str(),
                    "init_decl" | "blank_import" | "ambient_call" | "naked_go"
                );
                if cell_scoped && !in_cells {
                    continue;
                }
                let (uri, why, fix) = match kind.as_str() {
                    "init_decl" if !in_test => (
                        GO_GUIDE_CELLS,
                        "`init()` makes importing this package an execution",
                        "register in the composition root; keep the cell import-pure",
                    ),
                    "blank_import" if !in_test => (
                        GO_GUIDE_CELLS,
                        "a blank import exists only for its side effect",
                        "move driver-style registration to a boundary adapter",
                    ),
                    "ambient_call" if !in_test => (
                        GO_GUIDE_CELLS,
                        "an ambient default couples the cell to global state",
                        "inject the capability (a private narrow interface) at construction",
                    ),
                    "naked_go" if !in_test => (
                        GO_GUIDE_ERRORS,
                        "a naked `go` statement starts a goroutine nobody owns",
                        "own it: errgroup.Group / WaitGroup + context cancellation",
                    ),
                    "error_string_match" => (
                        GO_GUIDE_ERRORS,
                        "matching on an error's string couples to prose, not contract",
                        "consume the seam's closed error set via errors.As on its Code",
                    ),
                    "t_skip" if *in_test => (
                        GO_GUIDE_REPLACEMENT,
                        "`t.Skip` hides both regressions and healings",
                        "record the failure in discipline/registry/tests-baseline.json instead",
                    ),
                    "reasonless_suppression" => (
                        GO_GUIDE_BANS,
                        "a suppression without a reason is unrecorded testimony",
                        "append the reason (`//lint:ignore <Check> <reason>`), or fix the finding",
                    ),
                    "seam_error_missing_req" if !in_test => (
                        GO_GUIDE_ERRORS,
                        "a seam error type without a Spec field cannot cite its REQ",
                        "carry the violated spec:// URI (Code + Spec + Err) and render it",
                    ),
                    _ => continue,
                };
                out.push(Finding {
                    rule: self.id(),
                    file: source.file.clone(),
                    line: *line,
                    message: req_message(uri, why, fix),
                    why: self.why(),
                    fingerprint: format!("{}|{}|{kind}#{line}", self.id(), source.file),
                });
            }
        }
        out
    }
}

/// `go-cell-isolation` — R-002 over Go import paths: a file inside one
/// cell (a directory under `cells_dir`) may not import a SIBLING cell
/// at all. There is no seam-module exception (the TS shape): Go seams
/// live in a neutral package outside `cells_dir`, and the registry —
/// also outside — is the only cell importer, so any
/// `…/<cells_dir>/<other-cell>` import from inside a cell is a
/// violation. Imports outside `cells_dir` (seams, core, stdlib,
/// third-party) are free — the rule is about sibling privacy, not the
/// whole graph.
///
/// ```
/// use core_ai_native_conform::rules::GoCellIsolation;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let rule = GoCellIsolation::new("internal/cells");
/// let facts = vec![SourceFacts {
///     file: "internal/cells/naiveplanner/planner.go".into(),
///     crate_name: "demo".into(),
///     facts: vec![Fact::Import {
///         from_module: "internal/cells/naiveplanner/planner.go".into(),
///         to_path: "example.com/demo/internal/cells/batchplanner".into(),
///         line: 5,
///     }],
/// }];
/// let findings = rule.check(&facts);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].message.contains("batchplanner"));
/// ```
pub struct GoCellIsolation {
    cells_dir: String,
}

impl GoCellIsolation {
    pub fn new(cells_dir: &str) -> GoCellIsolation {
        GoCellIsolation {
            cells_dir: cells_dir.trim_matches('/').to_string(),
        }
    }

    /// The cell a repo-relative FILE path belongs to, if it is under
    /// `cells_dir`: `internal/cells/plan/plan.go` → `Some("plan")`.
    fn cell_of_file<'a>(&self, rel: &'a str) -> Option<&'a str> {
        let rest = rel.strip_prefix(self.cells_dir.as_str())?;
        let rest = rest.strip_prefix('/')?;
        let cell = rest.split('/').next()?;
        if cell.is_empty() { None } else { Some(cell) }
    }

    /// The cell an IMPORT path names, if any: Go import paths are
    /// module-qualified (`example.com/demo/internal/cells/plan`), so
    /// the cell is whatever follows the `cells_dir` segment.
    fn cell_of_import<'a>(&self, import: &'a str) -> Option<&'a str> {
        let needle = format!("{}/", self.cells_dir);
        let idx = import.find(&needle)?;
        // Guard against substring accidents: the match must sit at a
        // path-segment boundary.
        if idx > 0 && !import[..idx].ends_with('/') {
            return None;
        }
        let cell = import[idx + needle.len()..].split('/').next()?;
        if cell.is_empty() { None } else { Some(cell) }
    }
}

impl Rule for GoCellIsolation {
    fn id(&self) -> &'static str {
        "go-cell-isolation"
    }
    fn why(&self) -> &'static str {
        "a cell is the unit of modification, closed under paging; a sibling import \
         hides the dependency graph the pager needs and fuses two cells"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for source in facts {
            let Some(from_cell) = self.cell_of_file(&source.file) else {
                continue;
            };
            for fact in &source.facts {
                let Fact::Import { to_path, line, .. } = fact else {
                    continue;
                };
                let Some(target_cell) = self.cell_of_import(to_path) else {
                    continue;
                };
                if target_cell == from_cell {
                    continue;
                }
                out.push(Finding {
                    rule: self.id(),
                    file: source.file.clone(),
                    line: *line,
                    message: req_message(
                        GO_GUIDE_CELLS,
                        &format!(
                            "cell `{from_cell}` imports sibling cell `{target_cell}` \
                             (`{to_path}`)"
                        ),
                        "depend on the seams package instead, or move the shared piece \
                         into core; only the registry imports cells",
                    ),
                    why: self.why(),
                    fingerprint: format!("{}|{}|{to_path}#{line}", self.id(), source.file),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go_source(file: &str, facts: Vec<Fact>) -> SourceFacts {
        SourceFacts {
            file: file.into(),
            crate_name: "demo".into(),
            facts,
        }
    }

    #[test]
    fn deviation_reason_is_honoured_and_reasonless_suppression_is_not() {
        let facts = vec![go_source(
            "internal/cells/plan/plan.go",
            vec![
                Fact::GoUnsafe {
                    kind: "ambient_call".into(),
                    line: 3,
                    in_test: false,
                    reason: Some("wall clock is the domain here".into()),
                },
                Fact::GoUnsafe {
                    kind: "reasonless_suppression".into(),
                    line: 9,
                    in_test: false,
                    reason: None,
                },
            ],
        )];
        let findings = GoUnsafeInDomain::new(Some("internal/cells")).check(&facts);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 9);
    }

    #[test]
    fn value_bans_skip_test_files_but_t_skip_fires_only_there() {
        let facts = vec![go_source(
            "internal/cells/plan/plan_test.go",
            vec![
                Fact::GoUnsafe {
                    kind: "ambient_call".into(),
                    line: 1,
                    in_test: true,
                    reason: None,
                },
                Fact::GoUnsafe {
                    kind: "t_skip".into(),
                    line: 2,
                    in_test: true,
                    reason: None,
                },
            ],
        )];
        let findings = GoUnsafeInDomain::new(Some("internal/cells")).check(&facts);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("t.Skip"));
    }

    #[test]
    fn cell_scoped_kinds_stay_silent_outside_cells_dir() {
        let facts = vec![go_source(
            "cmd/reconcile/main.go",
            vec![
                Fact::GoUnsafe {
                    kind: "ambient_call".into(),
                    line: 4,
                    in_test: false,
                    reason: None,
                },
                Fact::GoUnsafe {
                    kind: "error_string_match".into(),
                    line: 8,
                    in_test: false,
                    reason: None,
                },
            ],
        )];
        let findings = GoUnsafeInDomain::new(Some("internal/cells")).check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("errors.As"));
    }

    #[test]
    fn sibling_import_fails_seams_and_own_cell_pass() {
        let rule = GoCellIsolation::new("internal/cells");
        let facts = vec![go_source(
            "internal/cells/naiveplanner/planner.go",
            vec![
                Fact::Import {
                    from_module: "internal/cells/naiveplanner/planner.go".into(),
                    to_path: "example.com/demo/internal/seams".into(),
                    line: 1,
                },
                Fact::Import {
                    from_module: "internal/cells/naiveplanner/planner.go".into(),
                    to_path: "example.com/demo/internal/cells/naiveplanner/sub".into(),
                    line: 2,
                },
                Fact::Import {
                    from_module: "internal/cells/naiveplanner/planner.go".into(),
                    to_path: "context".into(),
                    line: 3,
                },
                Fact::Import {
                    from_module: "internal/cells/naiveplanner/planner.go".into(),
                    to_path: "example.com/demo/internal/cells/batchplanner".into(),
                    line: 4,
                },
            ],
        )];
        let findings = rule.check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, 4);
        assert!(crate::rules::matches_req_grammar(&findings[0].message));
    }

    #[test]
    fn files_outside_cells_dir_import_cells_freely() {
        let rule = GoCellIsolation::new("internal/cells");
        let facts = vec![go_source(
            "internal/registry/registry.go",
            vec![Fact::Import {
                from_module: "internal/registry/registry.go".into(),
                to_path: "example.com/demo/internal/cells/batchplanner".into(),
                line: 5,
            }],
        )];
        assert!(rule.check(&facts).is_empty());
    }
}
