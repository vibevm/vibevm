//! The TypeScript rule family (GUIDE-AI-NATIVE-TYPESCRIPT §3, §8),
//! fed by the `ts-tsc` frontend's facts: the `unsafe`-set ban with its
//! recorded-deviation escape hatch, and cell isolation over import
//! specifiers. Defined ONCE here — the neutral engine — so the rule
//! cannot drift between language projections (the Ф6 brief's argument
//! for routing these through conform instead of ESLint).

specmark::scope!("spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};
use crate::rules::req_message;

const TS_GUIDE_BANS: &str = "discipline://typescript-ai-native-lang/guide#the-unsafe-set";
const TS_GUIDE_CELLS: &str = "discipline://typescript-ai-native-lang/guide#cells";

/// `ts-unsafe-in-domain` — the §8 ban set as Class-F findings: `any`
/// in type position, a cross-type `as`, a non-null `!`, `@ts-ignore`
/// always, and `@ts-expect-error` WITHOUT a `-- reason`. A reasoned
/// `@ts-expect-error -- reason` is recorded testimony (the TS shape of
/// `#[spec(deviates)]`) and is honoured, not flagged. Test files are
/// out of scope for the value-level bans (the guide scopes the ban to
/// domain code), but `@ts-ignore` stays banned everywhere — it rots
/// silently even in tests.
///
/// ```
/// use core_ai_native_conform::rules::TsUnsafeInDomain;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let facts = vec![SourceFacts {
///     file: "src/cells/parse/logic.ts".into(),
///     crate_name: "src".into(),
///     facts: vec![Fact::TsUnsafe {
///         kind: "any_type".into(),
///         line: 7,
///         in_test: false,
///         reason: None,
///     }],
/// }];
/// let findings = TsUnsafeInDomain.check(&facts);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].message.contains("any"));
/// ```
pub struct TsUnsafeInDomain;

impl Rule for TsUnsafeInDomain {
    fn id(&self) -> &'static str {
        "ts-unsafe-in-domain"
    }
    fn why(&self) -> &'static str {
        "types are erased and can be lied to; every unsafe-set token is a place the \
         runtime can diverge from the checked types"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for source in facts {
            for fact in &source.facts {
                let Fact::TsUnsafe {
                    kind,
                    line,
                    in_test,
                    reason,
                } = fact
                else {
                    continue;
                };
                let (why, fix) = match kind.as_str() {
                    "any_type" if !in_test => (
                        "`any` disables checking and propagates transitively",
                        "use `unknown` + a runtime narrowing, or record a deviation",
                    ),
                    "as_cross" if !in_test => (
                        "a cross-type `as` makes the compiler believe a lie",
                        "narrow with a runtime check first (`as const` is always fine)",
                    ),
                    "non_null" if !in_test => (
                        "`!` claims non-null without proof",
                        "narrow, or use an `asserts x is NonNullable<T>` function",
                    ),
                    "ts_ignore" => (
                        "`@ts-ignore` silences the compiler invisibly and cannot rot loudly",
                        "use `@ts-expect-error -- reason`, which fails when the error goes",
                    ),
                    "ts_expect_error" if reason.is_none() => (
                        "`@ts-expect-error` without `-- reason` is an unrecorded deviation",
                        "append `-- <why this suppression is sound>`",
                    ),
                    _ => continue,
                };
                out.push(Finding {
                    rule: self.id(),
                    file: source.file.clone(),
                    line: *line,
                    message: req_message(TS_GUIDE_BANS, why, fix),
                    why: self.why(),
                    fingerprint: format!("{}|{}|{kind}#{line}", self.id(), source.file),
                });
            }
        }
        out
    }
}

/// `ts-cell-isolation` — R-002 over import specifiers: a file inside
/// one cell (a directory under `cells_dir`) may import a SIBLING cell
/// only through its seam module (`<cell>/index.*`), never its
/// internals. Imports outside `cells_dir` (core, node builtins, npm
/// packages) are free — the rule is about sibling privacy, not the
/// whole graph.
///
/// ```
/// use core_ai_native_conform::rules::TsCellIsolation;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let rule = TsCellIsolation::new("src/cells", "index");
/// let facts = vec![SourceFacts {
///     file: "src/cells/parse/logic.ts".into(),
///     crate_name: "src".into(),
///     facts: vec![Fact::Import {
///         from_module: "src/cells/parse/logic.ts".into(),
///         to_path: "../greet/internal.js".into(),
///         line: 2,
///     }],
/// }];
/// let findings = rule.check(&facts);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].message.contains("internal"));
/// ```
pub struct TsCellIsolation {
    cells_dir: String,
    seam: String,
}

impl TsCellIsolation {
    pub fn new(cells_dir: &str, seam: &str) -> TsCellIsolation {
        TsCellIsolation {
            cells_dir: cells_dir.trim_matches('/').to_string(),
            seam: seam.to_string(),
        }
    }

    /// The cell a repo-relative path belongs to, if it is under
    /// `cells_dir`: `src/cells/greet/internal.ts` → `Some("greet")`.
    fn cell_of<'a>(&self, rel: &'a str) -> Option<&'a str> {
        let rest = rel.strip_prefix(self.cells_dir.as_str())?;
        let rest = rest.strip_prefix('/')?;
        let cell = rest.split('/').next()?;
        if cell.is_empty() { None } else { Some(cell) }
    }

    /// Resolve a relative import specifier against the importing file.
    /// Non-relative specifiers (bare packages, `node:` builtins) return
    /// `None` — out of the rule's scope.
    fn resolve(&self, from_file: &str, spec: &str) -> Option<String> {
        if !spec.starts_with("./") && !spec.starts_with("../") {
            return None;
        }
        let mut parts: Vec<&str> = from_file.split('/').collect();
        parts.pop(); // the importing file itself
        for comp in spec.split('/') {
            match comp {
                "." | "" => {}
                ".." => {
                    parts.pop()?;
                }
                other => parts.push(other),
            }
        }
        Some(parts.join("/"))
    }

    /// Whether a resolved import target is the seam module of its cell
    /// (`…/<cell>/index.ts`, any TS/JS extension or extensionless).
    fn is_seam(&self, target: &str, cell: &str) -> bool {
        let after = match target
            .split_once(&format!("{}/{cell}/", self.cells_dir))
            .map(|(_, after)| after)
        {
            Some(after) => after,
            None => return false,
        };
        let stem = after.split_once('.').map_or(after, |(stem, _)| stem);
        stem == self.seam
    }
}

impl Rule for TsCellIsolation {
    fn id(&self) -> &'static str {
        "ts-cell-isolation"
    }
    fn why(&self) -> &'static str {
        "a cell is the unit of modification, closed under paging; reaching into a \
         sibling's internals hides the dependency graph the pager needs"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for source in facts {
            let Some(from_cell) = self.cell_of(&source.file) else {
                continue;
            };
            for fact in &source.facts {
                let Fact::Import { to_path, line, .. } = fact else {
                    continue;
                };
                let Some(target) = self.resolve(&source.file, to_path) else {
                    continue;
                };
                let Some(target_cell) = self.cell_of(&target) else {
                    continue;
                };
                if target_cell == from_cell || self.is_seam(&target, target_cell) {
                    continue;
                }
                out.push(Finding {
                    rule: self.id(),
                    file: source.file.clone(),
                    line: *line,
                    message: req_message(
                        TS_GUIDE_CELLS,
                        &format!(
                            "cell `{from_cell}` imports sibling `{target_cell}` internals \
                             (`{to_path}`)"
                        ),
                        &format!(
                            "import `{}/{target_cell}/{}` (the seam), or move the shared \
                             piece into core",
                            self.cells_dir, self.seam
                        ),
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

    fn ts_source(file: &str, facts: Vec<Fact>) -> SourceFacts {
        SourceFacts {
            file: file.into(),
            crate_name: "src".into(),
            facts,
        }
    }

    #[test]
    fn reasoned_expect_error_is_honoured_and_unreasoned_is_not() {
        let facts = vec![ts_source(
            "src/a.ts",
            vec![
                Fact::TsUnsafe {
                    kind: "ts_expect_error".into(),
                    line: 3,
                    in_test: false,
                    reason: Some("narrowed upstream".into()),
                },
                Fact::TsUnsafe {
                    kind: "ts_expect_error".into(),
                    line: 9,
                    in_test: false,
                    reason: None,
                },
            ],
        )];
        let findings = TsUnsafeInDomain.check(&facts);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 9);
    }

    #[test]
    fn value_bans_skip_test_files_but_ts_ignore_never_does() {
        let facts = vec![ts_source(
            "src/a.test.ts",
            vec![
                Fact::TsUnsafe {
                    kind: "any_type".into(),
                    line: 1,
                    in_test: true,
                    reason: None,
                },
                Fact::TsUnsafe {
                    kind: "ts_ignore".into(),
                    line: 2,
                    in_test: true,
                    reason: None,
                },
            ],
        )];
        let findings = TsUnsafeInDomain.check(&facts);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("@ts-ignore"));
    }

    #[test]
    fn seam_imports_and_core_imports_pass_internals_fail() {
        let rule = TsCellIsolation::new("src/cells", "index");
        let facts = vec![ts_source(
            "src/cells/parse/logic.ts",
            vec![
                Fact::Import {
                    from_module: "src/cells/parse/logic.ts".into(),
                    to_path: "../greet/index.js".into(),
                    line: 1,
                },
                Fact::Import {
                    from_module: "src/cells/parse/logic.ts".into(),
                    to_path: "../../core/util.js".into(),
                    line: 2,
                },
                Fact::Import {
                    from_module: "src/cells/parse/logic.ts".into(),
                    to_path: "node:fs".into(),
                    line: 3,
                },
                Fact::Import {
                    from_module: "src/cells/parse/logic.ts".into(),
                    to_path: "./helper.js".into(),
                    line: 4,
                },
                Fact::Import {
                    from_module: "src/cells/parse/logic.ts".into(),
                    to_path: "../greet/internal.js".into(),
                    line: 5,
                },
            ],
        )];
        let findings = rule.check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, 5);
        assert!(core_ai_native_conform_grammar_ok(&findings[0].message));
    }

    fn core_ai_native_conform_grammar_ok(message: &str) -> bool {
        crate::rules::matches_req_grammar(message)
    }
}
