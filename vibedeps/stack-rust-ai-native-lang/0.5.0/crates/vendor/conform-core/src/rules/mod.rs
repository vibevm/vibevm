//! The standing conform rules, one file per family: structure rules
//! (R-001 flag-sites, R-002 cell isolation, the Class-D
//! cell-has-oracle net), diagnostics rules (the Class-G seam-doctest
//! gate and the two Class-F REQ-citation halves), and budget-and-bans
//! rules (unsafe-gate, file-length, no-unwrap-in-domain). The Class-F
//! message grammar (`req_message` / `matches_req_grammar`) and the
//! shared `#[cell]` discovery helper live here; every rule type keeps
//! its public path `conform_core::rules::<RuleType>` via the
//! re-exports below.

specmark::scope!("spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};

mod budget;
mod diagnostics;
mod structure;
mod typescript;

pub use budget::{AmbientEnv, FileLength, NoUnwrapInDomain, UnsafeGate};
pub use diagnostics::{ErrorEnumCitesReq, ErrorMessageCitesReq, PubDoctest, SeamHasDoctest};
pub use structure::{CellHasOracle, CellIsolation, FlagSites};
pub use typescript::{TsCellIsolation, TsUnsafeInDomain};

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
///     "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
///     "what went wrong",
///     "where to fix it",
/// );
/// assert_eq!(
///     msg,
///     "violates REQ spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules: \
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

/// True when `file` (repo-relative, forward slashes) lies under a
/// crate's `src/` tree — both the workspace shape (`crates/x/src/…`)
/// and the bare single-crate shape (`src/…`, where the crate dir IS the
/// project root under `roots = ["."]` and paths carry no crate prefix).
/// Every path-scoped rule filters through this one predicate; an inline
/// `contains("/src/")` silently skips the whole bare shape.
pub(crate) fn in_src(file: &str) -> bool {
    file.starts_with("src/") || file.contains("/src/")
}

/// The `tests/` twin of [`in_src`].
pub(crate) fn in_tests(file: &str) -> bool {
    file.starts_with("tests/") || file.contains("/tests/")
}

/// True when `file` is its crate's `src/lib.rs`, in either shape.
pub(crate) fn is_lib_root(file: &str) -> bool {
    file == "src/lib.rs" || file.ends_with("/src/lib.rs")
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

#[cfg(test)]
mod tests;
