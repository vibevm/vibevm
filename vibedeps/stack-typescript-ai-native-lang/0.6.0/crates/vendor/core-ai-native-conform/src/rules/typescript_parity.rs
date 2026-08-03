//! The TypeScript parity rule family (B-033): a seam's error union
//! carries its REQ URI (`ts-seam-error-cites-req`). Split out of
//! `typescript.rs` along the parity seam so neither file crosses the
//! 600-line budget; the scope unit matches its siblings so self-trace
//! finds no orphan.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};
use crate::rules::req_message;

const TS_GUIDE_SEAM_ERROR: &str =
    "discipline://typescript-ai-native-lang/guide#rule-failure-on-a-seam-is-a-typed-value";

/// `ts-seam-error-cites-req` — the TS twin of Rust's
/// `error-enum-cites-req` / `error-message-cites-req` and Go's
/// `go-seam-error-cites-req` (GUIDE-AI-NATIVE-TYPESCRIPT §6, B-033):
/// failure on a seam is a typed value — a discriminated-union error
/// alias `E` whose variants carry `spec://` REQ references — so a
/// failing run is navigable back to the requirement. The rule fires
/// when the extractor's computed `cites_req` flag is false: the union
/// cites no `spec://` REQ (neither a JSDoc `@implements`/`@documents`
/// marker on the alias nor a `spec://` substring in a variant member).
/// Test files are out of scope (file-grain `in_test`).
///
/// **Honest limit, recorded not claimed.** Detection lives in the
/// `ts-tsc` extractor (`Fact::TsSeamError` is its signal); the engine
/// rule is a pure query over that flag. The exact discriminated-union
/// heuristic is the extractor's measured refinement point; whatever it
/// cannot see is recorded as a documented limit, never silently claimed
/// (the `ts-flag-sites` precedent).
///
/// ```
/// use core_ai_native_conform::rules::TsSeamErrorCitesReq;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let facts = vec![SourceFacts {
///     file: "src/cells/plan/error.ts".into(),
///     crate_name: "src".into(),
///     facts: vec![Fact::TsSeamError {
///         symbol: "PlanError".into(),
///         cites_req: false,
///         line: 4,
///         in_test: false,
///     }],
/// }];
/// let findings = TsSeamErrorCitesReq.check(&facts);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].message.contains("PlanError"));
/// ```
pub struct TsSeamErrorCitesReq;

impl Rule for TsSeamErrorCitesReq {
    fn id(&self) -> &'static str {
        "ts-seam-error-cites-req"
    }
    fn why(&self) -> &'static str {
        "failure on a seam is a typed value: the error union E is a \
         discriminated union of named variants carrying spec:// REQ references, \
         so a failing run is navigable back to the requirement \
         (GUIDE-AI-NATIVE-TYPESCRIPT §6)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for source in facts {
            for fact in &source.facts {
                let Fact::TsSeamError {
                    symbol,
                    cites_req,
                    line,
                    in_test,
                } = fact
                else {
                    continue;
                };
                if *cites_req || *in_test {
                    continue;
                }
                out.push(Finding {
                    rule: self.id(),
                    file: source.file.clone(),
                    line: *line,
                    message: req_message(
                        TS_GUIDE_SEAM_ERROR,
                        &format!("TS error union `{symbol}` cites no spec:// REQ"),
                        "carry the governing spec:// REQ on the union (a JSDoc \
                         @implements marker, or a spec:// substring in a variant)",
                    ),
                    why: self.why(),
                    fingerprint: format!("{}|{}|{symbol}", self.id(), source.file),
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
    fn ts_seam_error_not_citing_req_is_flagged() {
        let facts = vec![ts_source(
            "src/cells/plan/error.ts",
            vec![Fact::TsSeamError {
                symbol: "PlanError".into(),
                cites_req: false,
                line: 4,
                in_test: false,
            }],
        )];
        let findings = TsSeamErrorCitesReq.check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("PlanError"));
        assert!(findings[0].fingerprint.contains("PlanError"));
        assert!(core_ai_native_conform_grammar_ok(&findings[0].message));
    }

    #[test]
    fn ts_seam_error_citing_req_is_silent() {
        let facts = vec![ts_source(
            "src/cells/plan/error.ts",
            vec![Fact::TsSeamError {
                symbol: "PlanError".into(),
                cites_req: true,
                line: 4,
                in_test: false,
            }],
        )];
        assert!(TsSeamErrorCitesReq.check(&facts).is_empty());
    }

    #[test]
    fn ts_seam_error_in_test_file_is_out_of_scope() {
        let facts = vec![ts_source(
            "src/cells/plan/error.test.ts",
            vec![Fact::TsSeamError {
                symbol: "PlanError".into(),
                cites_req: false,
                line: 4,
                in_test: true,
            }],
        )];
        assert!(TsSeamErrorCitesReq.check(&facts).is_empty());
    }

    fn core_ai_native_conform_grammar_ok(message: &str) -> bool {
        crate::rules::matches_req_grammar(message)
    }
}
