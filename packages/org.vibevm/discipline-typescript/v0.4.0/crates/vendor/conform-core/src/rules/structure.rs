//! Structure rules — where cells are constructed and what a cell may
//! import: R-001 flag-sites, R-002 cell isolation, and the Class-D
//! cell-has-oracle replacement net.

specmark::scope!("spec://discipline-core/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};

use super::{cell_types, req_message};

/// R-001 — flag-sites: cell constructors appear only in the
/// selection registry module.
///
/// ```
/// use conform_core::rules::FlagSites;
/// use conform_core::Rule;
///
/// let rule = FlagSites {
///     registry_file: "crates/app/src/registry.rs".into(),
///     gated_crate: "app".into(),
/// };
/// assert_eq!(rule.id(), "R-001");
/// assert!(rule.check(&[]).is_empty());
/// ```
pub struct FlagSites {
    /// Repo-relative path of the one legal construction site.
    pub registry_file: String,
    /// The crate whose construction sites are gated.
    pub gated_crate: String,
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
            let referenced = facts
                .iter()
                .filter(|sf| sf.crate_name == crate_name && super::in_tests(&sf.file))
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
                    "discipline://rust-ai-native/cards/scaffold-d-differential-oracle#ops",
                    &format!(
                        "cell `{type_name}` is referenced by no integration test \
                         in its crate — it has no behavior oracle"
                    ),
                    &format!(
                        "add a differential or characterization test in \
                         `{crate_name}`'s tests/ that drives `{type_name}`"
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
