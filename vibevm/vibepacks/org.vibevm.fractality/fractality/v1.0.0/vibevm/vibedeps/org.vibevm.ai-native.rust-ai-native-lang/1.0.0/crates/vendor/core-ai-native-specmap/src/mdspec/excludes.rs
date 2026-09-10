//! The enumerable markdown exclusion ([`Config::spec_exclude`]) — compiled
//! patterns with a per-pattern hit count, applied to **both** halves of
//! [`scan_spec_tree`](super::scan_spec_tree) (the `spec_roots` walk and the
//! `root_spec_docs` list) so a match leaves the inventory before it is parsed
//! into a unit. Split out of `mdspec.rs` for the file-length budget, by the
//! same seam as `mdspec/lines.rs`.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use crate::config::Config;
use crate::generated::specmap::Warning;

/// The compiled [`Config::spec_exclude`] patterns with a per-pattern hit
/// count, so a pattern that matches nothing can be reported rather than
/// tolerated. Mirrors the progress gate's `apply_config_excludes` /
/// `ExcludeReport` invariant (this engine and that one are separate crates,
/// so the *invariant* is ported, not the code):
///
/// * every pattern is tested against every candidate path, never
///   short-circuited on the first hit — "matched nothing" is a per-pattern
///   fact, and a pattern that only ever overlaps another is still doing work
///   (so it must not be falsely called stale);
/// * a pattern that is not a valid glob is reported by name, never panicked
///   over and never silently skipped — a skip would leave the corpus wider
///   than the config says;
/// * a pattern that matched no file is reported verbatim, so a reader can
///   delete it without guessing.
pub(super) struct SpecExcludes {
    /// `(verbatim pattern, compiled)` for each pattern that parsed.
    compiled: Vec<(String, glob::Pattern)>,
    /// One hit counter per compiled pattern — candidate files it matched. A
    /// pattern is stale at `0`.
    hits: Vec<usize>,
}

impl SpecExcludes {
    /// Compile every pattern. Each pattern that fails to parse becomes a
    /// `bad-exclude-glob` warning naming it verbatim (and is dropped from the
    /// compiled set — it cannot match, so it is not stale, only broken).
    pub(super) fn compile(patterns: &[String]) -> (SpecExcludes, Vec<Warning>) {
        let mut compiled = Vec::new();
        let mut warnings = Vec::new();
        for p in patterns {
            match glob::Pattern::new(p) {
                Ok(c) => compiled.push((p.clone(), c)),
                Err(e) => warnings.push(Warning {
                    code: "bad-exclude-glob".to_string(),
                    message: format!("exclude pattern `{p}` is not a valid glob: {e}"),
                    file: Config::REL_PATH.to_string(),
                    line: 0,
                }),
            }
        }
        let hits = vec![0; compiled.len()];
        (SpecExcludes { compiled, hits }, warnings)
    }

    /// Does `file_rel` match any exclude? Every pattern is tested — no
    /// short-circuit — so a pattern overlapping another still registers its
    /// own hit and is not later misreported as stale. `file_rel` is the
    /// `/`-separated repo-relative path: the exact string a `SpecUnit` carries
    /// as `file`, whichever include half surfaced it.
    pub(super) fn matches(&mut self, file_rel: &str) -> bool {
        let mut excluded = false;
        for (i, (_, pat)) in self.compiled.iter().enumerate() {
            if pat.matches(file_rel) {
                self.hits[i] += 1;
                excluded = true;
            }
        }
        excluded
    }

    /// One `stale-exclude` warning per pattern that matched no candidate file
    /// — naming the pattern verbatim, so it can be removed without guessing.
    pub(super) fn stale_warnings(self) -> Vec<Warning> {
        self.compiled
            .iter()
            .zip(&self.hits)
            .filter(|(_, n)| **n == 0)
            .map(|((p, _), _)| Warning {
                code: "stale-exclude".to_string(),
                message: format!(
                    "exclude pattern `{p}` matched no spec file — a stale \
                     exclusion protects nothing; remove it from specmap.toml"
                ),
                file: Config::REL_PATH.to_string(),
                line: 0,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    //! Enumerable-exclusion tests, kept beside the mechanism (the rest of the
    //! markdown-scanner tests live in `mdspec/tests.rs`).
    use super::super::scan_spec_tree;
    use crate::config::Config;
    use crate::generated::specmap::Warning;

    fn fmt_warnings(w: &[Warning]) -> String {
        w.iter()
            .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// A `vibevm/vibespecs/` tree with one kept doc beside one the project excludes: the
    /// kind of split a `spec_roots` glob cannot name on its own ("everything
    /// under `vibevm/vibespecs/`, except the rewritten-every-session checkpoint"). Two
    /// units live under the excluded file (`cp-root`, `cp-daily`) and one
    /// under the kept neighbour — three in the unexcluded corpus.
    fn spec_exclude_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let spec = dir.path().join("vibevm/vibespecs");
        std::fs::create_dir_all(spec.join("kept")).unwrap();
        std::fs::create_dir_all(spec.join("gen")).unwrap();
        std::fs::write(spec.join("kept/DOC.md"), "## Kept {#kept-unit}\nbody\n").unwrap();
        std::fs::write(
            spec.join("gen/WAL.md"),
            "# Checkpoint {#cp-root}\n\n## Daily {#cp-daily}\nbody\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn an_excluded_file_yields_no_units_but_its_neighbour_does() {
        let dir = spec_exclude_tree();
        let cfg = Config {
            spec_exclude: vec!["vibevm/vibespecs/gen/WAL.md".into()],
            ..Config::default()
        };
        let (units, warnings) = scan_spec_tree(dir.path(), &cfg);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        let uris: Vec<&str> = units.iter().map(|u| u.uri.as_str()).collect();
        assert!(uris.contains(&"spec://project/kept/DOC#kept-unit"));
        // The excluded file is gone entirely — root heading and subsection both.
        assert!(!uris.contains(&"spec://project/gen/WAL#cp-root"));
        assert!(!uris.contains(&"spec://project/gen/WAL#cp-daily"));
        assert_eq!(units.len(), 1);
    }

    #[test]
    fn spec_exclude_applies_to_root_spec_docs_too() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("vibevm/vibespecs")).unwrap();
        std::fs::write(
            dir.path().join("vibevm/vibespecs/IN.md"),
            "## In tree {#in-tree}\nbody\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("FROZEN.md"),
            "# demo {#root}\n\n## Section 5. {#task-graph}\nbody\n",
        )
        .unwrap();
        // FROZEN.md is both named in root_spec_docs and matched by spec_exclude.
        // The exclude is tested against the root-doc half on the same `file`
        // string a unit would carry, so it wins uniformly with the spec-roots
        // half; a matching pattern is not stale, so it stays silent here.
        let cfg = Config {
            root_spec_docs: vec!["FROZEN.md".into()],
            spec_exclude: vec!["FROZEN.md".into()],
            ..Config::default()
        };
        let (units, warnings) = scan_spec_tree(dir.path(), &cfg);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        let uris: Vec<&str> = units.iter().map(|u| u.uri.as_str()).collect();
        assert!(uris.contains(&"spec://project/IN#in-tree"));
        assert!(!uris.contains(&"spec://project/FROZEN#root"));
        assert!(!uris.contains(&"spec://project/FROZEN#task-graph"));
        assert_eq!(units.len(), 1);
    }

    #[test]
    fn a_stale_exclude_pattern_names_itself_and_removes_nothing() {
        let dir = spec_exclude_tree();
        let cfg = Config {
            spec_exclude: vec!["vibevm/vibespecs/gen/RETIRED.md".into()],
            ..Config::default()
        };
        let (units, warnings) = scan_spec_tree(dir.path(), &cfg);
        // Nothing matched, so the full corpus is inventoried.
        assert_eq!(units.len(), 3, "a stale exclude removes nothing");
        let stale: Vec<&Warning> = warnings
            .iter()
            .filter(|w| w.code == "stale-exclude")
            .collect();
        assert_eq!(stale.len(), 1, "{}", fmt_warnings(&warnings));
        // The warning names the pattern verbatim — a reader can delete it
        // whole, without guessing which line in specmap.toml it was.
        assert!(
            stale[0].message.contains("vibevm/vibespecs/gen/RETIRED.md"),
            "{}",
            stale[0].message
        );
    }

    #[test]
    fn an_invalid_exclude_glob_is_a_warning_naming_the_pattern() {
        let dir = spec_exclude_tree();
        let cfg = Config {
            spec_exclude: vec!["vibevm/vibespecs/a**/x.md".into()],
            ..Config::default()
        };
        let (units, warnings) = scan_spec_tree(dir.path(), &cfg);
        // The bad glob is reported by name — never panicked over, never
        // silently skipped (a skip would leave the corpus wider than the
        // config says).
        let bad: Vec<&Warning> = warnings
            .iter()
            .filter(|w| w.code == "bad-exclude-glob")
            .collect();
        assert_eq!(bad.len(), 1, "{}", fmt_warnings(&warnings));
        assert!(
            bad[0].message.contains("vibevm/vibespecs/a**/x.md"),
            "{}",
            bad[0].message
        );
        // It never compiled, so it is not also misreported as stale; and the
        // tree it failed to prune stays intact — which is exactly why it is
        // shouted about rather than tolerated.
        assert!(warnings.iter().all(|w| w.code != "stale-exclude"));
        assert_eq!(units.len(), 3);
    }

    #[test]
    fn an_empty_spec_exclude_changes_nothing() {
        let dir = spec_exclude_tree();
        // Default config: spec_exclude is empty, so the scan is the pre-field
        // scan verbatim — every unit present, no exclude-shaped warning.
        let (units, warnings) = scan_spec_tree(dir.path(), &Config::default());
        assert_eq!(units.len(), 3);
        assert!(
            warnings
                .iter()
                .all(|w| !matches!(w.code.as_str(), "stale-exclude" | "bad-exclude-glob")),
            "{}",
            fmt_warnings(&warnings)
        );
    }
}
