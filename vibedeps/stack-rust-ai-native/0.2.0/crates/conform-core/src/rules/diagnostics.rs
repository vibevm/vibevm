//! Diagnostics rules — Class G and Class F: every public seam shows
//! one compiled doctest, and error surfaces stay navigable back to
//! their REQ units (the attribute half and the message half, the gap
//! audit 2026-06-12-08 recorded).

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};

use super::req_message;

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
/// let rule = SeamHasDoctest { gated_crates: vec!["x".into()] };
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
    pub gated_crates: Vec<String>,
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
            if !self.gated_crates.contains(&sf.crate_name) {
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
                        "discipline://rust-ai-native/cards/scaffold-g-doctests#ops",
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

/// Class G — `pub-doctest`: every public *type* seam (`struct`,
/// `enum`, `trait`, `union`) declared under `src/` in a gated crate
/// carries at least one compiled doc example, or a `#[spec(documents)]`
/// edge to a guide unit. Where `seam-has-doctest` gates the crate-root
/// surface plus traits, this widens the lens to the whole declared
/// public type API — the foundation crate re-exports its types from
/// submodules (and re-exports are not item facts), so those definitions
/// were previously unseen. It activates on the foundation crate first,
/// freezing its accumulated doc-debt and shrinking from there
/// (card scaffold-g-doctests; CONVERT-PLAN v0.1 §2 item 1.4).
///
/// Scoped to type seams rather than every `pub` symbol deliberately:
/// a type is the unit a reader pages in to learn the crate; a free
/// `fn` or `const` is reached *through* a type and is covered where it
/// matters by `seam-has-doctest`'s crate-root lens. This keeps the gate
/// on the items that teach and off the trivia.
///
/// ```
/// use conform_core::rules::PubDoctest;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let rule = PubDoctest { gated_crates: vec!["x".into()] };
/// let sub = SourceFacts {
///     file: "crates/x/src/types.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![Fact::Item {
///         kind: "struct".into(), symbol: "x::types::Entry".into(), line: 7,
///         attrs: vec![], is_pub: true, has_doctest: false,
///     }],
/// };
/// assert_eq!(rule.check(&[sub]).len(), 1);
/// ```
pub struct PubDoctest {
    pub gated_crates: Vec<String>,
}

impl Rule for PubDoctest {
    fn id(&self) -> &'static str {
        "pub-doctest"
    }
    fn why(&self) -> &'static str {
        "the foundation crate is the few-shot prompt a model copies first: \
         every public type it pages in teaches by one compiled example, not \
         prose alone (card scaffold-g-doctests; CONVERT-PLAN v0.1 §2.4)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if !self.gated_crates.contains(&sf.crate_name) || !sf.file.contains("/src/") {
                continue;
            }
            for f in &sf.facts {
                let Fact::Item {
                    kind,
                    symbol,
                    line,
                    attrs,
                    is_pub,
                    has_doctest,
                } = f
                else {
                    continue;
                };
                if !is_pub || *has_doctest {
                    continue;
                }
                if !matches!(kind.as_str(), "struct" | "enum" | "trait" | "union") {
                    continue;
                }
                // A `#[spec(documents = …)]` edge is the prose-free alternative
                // to a compiled example.
                if attrs.iter().any(|a| a.contains("documents")) {
                    continue;
                }
                let name = symbol.rsplit("::").next().unwrap_or(symbol);
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: *line,
                    message: req_message(
                        "discipline://rust-ai-native/cards/scaffold-g-doctests#ops",
                        &format!("public {kind} `{name}` has no compiled doctest"),
                        &format!(
                            "add one doctest on `{name}` showing canonical use, or a \
                             #[spec(documents = \"…\")] edge"
                        ),
                    ),
                    why: self.why(),
                    fingerprint: format!("pub-doctest|{}|{symbol}", sf.file),
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
/// let rule = ErrorMessageCitesReq { gated_crates: vec!["x".into()] };
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
    pub gated_crates: Vec<String>,
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
            if !self.gated_crates.contains(&sf.crate_name) {
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
                        "discipline://rust-ai-native/cards/scaffold-f-structured-diagnostics#ops",
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
/// let rule = ErrorEnumCitesReq { gated_crates: vec!["x".into()] };
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
    pub gated_crates: Vec<String>,
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
            if !self.gated_crates.contains(&sf.crate_name) {
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
                        "discipline://rust-ai-native/cards/scaffold-f-structured-diagnostics#ops",
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
