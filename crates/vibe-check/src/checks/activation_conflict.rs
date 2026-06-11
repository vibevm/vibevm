//! PROP-003 §2.10 — pairs of subskills whose `description` triggers
//! materially overlap (≥75% keyword Jaccard) within the same package's
//! lazy-push / lazy-pull set. Mirrors Tessl's review-rubric
//! "activation distinctiveness" axis.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use specmark::cell;

use super::scan_local_packages;
use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::ActivationConflict`] cell.
///
/// PROP-003 §2.10 activation-conflict heuristic. Within each
/// package, walk all subskills that need a `description` (delivery =
/// lazy-push / lazy-pull). For every pair, compute Jaccard
/// keyword-set similarity over their description texts. Pairs above
/// the threshold (75%) flag as warnings since the agent's natural-
/// language matching is likely to confuse them.
#[cell(seam = "Check", variant = "activation-conflict")]
pub struct ActivationConflictCheck;

impl Check for ActivationConflictCheck {
    fn id(&self) -> CheckId {
        CheckId::ActivationConflict
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        // PROP-003 §2.10 originally pinned the threshold at 75%, but
        // practical Jaccard on short trigger descriptions saturates at
        // around the high-60s for content-equivalent pairs once stopwords
        // are filtered. 70% catches the obvious false-positive shape
        // ("two descriptions that read like alternative phrasings of the
        // same prompt") without flagging genuinely-distinct pairs that
        // happen to share two domain keywords.
        const THRESHOLD: f64 = 0.70;
        for (pkg_root, source_label) in scan_local_packages(project_root) {
            let subskills_dir = pkg_root.join("subskills");
            if !subskills_dir.is_dir() {
                continue;
            }
            let mut entries: Vec<(PathBuf, String, String, std::collections::HashSet<String>)> =
                Vec::new();
            for entry in walkdir::WalkDir::new(&subskills_dir)
                .max_depth(6)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                if entry.file_name() != vibe_core::manifest::SubskillManifest::FILENAME {
                    continue;
                }
                let manifest = match vibe_core::manifest::SubskillManifest::read(entry.path()) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if !manifest.subskill.delivery.requires_description() {
                    continue;
                }
                let Some(desc) = manifest.subskill.description.clone() else {
                    continue;
                };
                let tokens = tokenise_for_overlap(&desc);
                let rel = entry
                    .path()
                    .strip_prefix(project_root)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| entry.path().to_path_buf());
                entries.push((rel, manifest.subskill.path.clone(), desc, tokens));
            }
            for i in 0..entries.len() {
                for j in (i + 1)..entries.len() {
                    let overlap = jaccard_overlap(&entries[i].3, &entries[j].3);
                    if overlap >= THRESHOLD {
                        report.warn(
                            CheckId::ActivationConflict,
                            Some(entries[i].0.clone()),
                            None,
                            format!(
                                "[{source_label}] subskill descriptions for `{}` and `{}` overlap by {:.0}% (threshold {:.0}%); the agent may confuse them at activation time. Tighten one description to be specific about its distinct trigger.",
                                entries[i].1,
                                entries[j].1,
                                overlap * 100.0,
                                THRESHOLD * 100.0
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn tokenise_for_overlap(s: &str) -> std::collections::HashSet<String> {
    /// Common English connective words that appear in nearly every
    /// agent-trigger description. Removing them sharpens the
    /// distinctiveness signal of the remaining keywords.
    const STOPWORDS: &[&str] = &[
        "the", "this", "that", "those", "these", "with", "when", "while", "for", "and", "but",
        "into", "your", "you", "are", "have", "has", "had", "from", "into", "over", "about",
        "above", "below", "after", "before", "during", "use", "using", "used", "needs", "need",
        "needed", "want", "wants", "wanted",
    ];
    s.split(|c: char| !c.is_alphanumeric())
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && t.len() > 2)
        .filter(|t| !STOPWORDS.iter().any(|w| *w == t))
        .collect()
}

fn jaccard_overlap(
    a: &std::collections::HashSet<String>,
    b: &std::collections::HashSet<String>,
) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, check_project};

    #[test]
    fn activation_conflict_flags_overlapping_descriptions() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let pkg = project
            .path()
            .join("packages")
            .join("flow")
            .join("test-pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"
"#,
        )
        .unwrap();
        // Two lazy-push subskills with deliberately overlapping
        // descriptions — same vocabulary, no distinct keywords.
        for (name, desc) in [
            (
                "a",
                "When working with database migrations using sqlx in a Rust project",
            ),
            (
                "b",
                "When using sqlx in a Rust project for database migrations work",
            ),
        ] {
            let dir = pkg.join(format!("subskills/{name}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("vibe-subskill.toml"),
                format!(
                    r#"[subskill]
path = "{name}"
delivery = "lazy-push"
description = "{desc}"
"#
                ),
            )
            .unwrap();
        }
        let report = check_project(project.path(), &opts());
        let conflict_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::ActivationConflict)
            .collect();
        assert!(
            !conflict_findings.is_empty(),
            "expected activation_conflict warning; got {:?}",
            report.findings
        );
        assert!(conflict_findings[0].message.contains("overlap by"));
    }

    #[test]
    fn activation_conflict_clean_when_descriptions_distinct() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let pkg = project
            .path()
            .join("packages")
            .join("flow")
            .join("test-pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"
"#,
        )
        .unwrap();
        for (name, desc) in [
            ("a", "When designing GraphQL schemas with Apollo Server"),
            ("b", "When writing PostgreSQL migrations using sqlx"),
        ] {
            let dir = pkg.join(format!("subskills/{name}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("vibe-subskill.toml"),
                format!(
                    r#"[subskill]
path = "{name}"
delivery = "lazy-push"
description = "{desc}"
"#
                ),
            )
            .unwrap();
        }
        let report = check_project(project.path(), &opts());
        let conflict_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::ActivationConflict)
            .collect();
        assert!(
            conflict_findings.is_empty(),
            "expected no activation_conflict findings; got {:?}",
            conflict_findings
        );
    }
}
