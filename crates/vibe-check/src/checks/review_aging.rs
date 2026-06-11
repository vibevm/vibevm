//! Check 9 — every `<!-- REVIEW: YYYY-MM-DD ... -->` marker in
//! `spec/**/*.md` whose date is older than `review_max_age_days`
//! (default 14) is reported as a warning.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;

use crate::{Check, CheckId, CheckOptions, CheckReport, Finding, Severity};

const REVIEW_MARKER_PREFIX: &str = "<!-- REVIEW";

/// The [`CheckId::ReviewAging`] cell.
#[cell(seam = "Check", variant = "review-aging")]
pub struct ReviewAgingCheck;

impl Check for ReviewAgingCheck {
    fn id(&self) -> CheckId {
        CheckId::ReviewAging
    }

    fn run(&self, project_root: &Path, opts: &CheckOptions, report: &mut CheckReport) {
        let now_unix = opts.now_unix_utc.unwrap_or_else(crate::unix_now_utc);
        let max_age_days = opts.review_max_age_days;
        let spec_dir = project_root.join("spec");
        if !spec_dir.is_dir() {
            return;
        }
        let max_age_secs = max_age_days * 86_400;
        for path in walk_files(&spec_dir) {
            let lossy = path.to_string_lossy();
            if !lossy.ends_with(".md") {
                continue;
            }
            let body = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let rel = match path.strip_prefix(project_root) {
                Ok(r) => normalize(r),
                Err(_) => path.clone(),
            };
            for (idx, line) in body.lines().enumerate() {
                let pos = match find_marker(line) {
                    Some(p) => p,
                    None => continue,
                };
                let after = &line[pos + REVIEW_MARKER_PREFIX.len()..];
                // Accept "<!-- REVIEW: YYYY-MM-DD …" (canonical) and
                // "<!-- REVIEW YYYY-MM-DD …" (legacy / sloppy).
                let after = after.trim_start_matches(':').trim_start();
                let date_field = after.split_whitespace().next().unwrap_or("");
                // Only treat the line as a real marker if the first
                // token is date-shaped. Documentation / prose mentions
                // of `<!-- REVIEW: ... -->` (init's user-facing
                // template, the spec itself describing the convention)
                // are skipped silently — they aren't tracking work.
                if !looks_like_date(date_field) {
                    continue;
                }
                let Some(date_unix) = parse_iso_date(date_field) else {
                    report.warn(
                        CheckId::ReviewAging,
                        Some(rel.clone()),
                        Some(idx + 1),
                        format!(
                            "REVIEW marker date `{date_field}` is malformed — expected `YYYY-MM-DD`"
                        ),
                    );
                    continue;
                };
                if date_unix > now_unix {
                    report.push(Finding {
                        check: CheckId::ReviewAging,
                        severity: Severity::Info,
                        path: Some(rel.clone()),
                        line: Some(idx + 1),
                        message: "REVIEW marker date is in the future — clock skew or typo".into(),
                    });
                    continue;
                }
                let age_secs = now_unix - date_unix;
                if age_secs > max_age_secs {
                    let age_days = age_secs / 86_400;
                    report.warn(
                        CheckId::ReviewAging,
                        Some(rel.clone()),
                        Some(idx + 1),
                        format!(
                            "REVIEW marker dated {date_field} is {age_days} days old (threshold: {max_age_days}d) — resolve or refresh"
                        ),
                    );
                }
            }
        }
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                out.push(path);
            }
        }
    }
    out
}

fn normalize(p: &Path) -> PathBuf {
    PathBuf::from(p.to_string_lossy().replace('\\', "/"))
}

fn find_marker(line: &str) -> Option<usize> {
    line.find(REVIEW_MARKER_PREFIX)
}

/// Heuristic for "this token starts with a `YYYY-MM-DD`-shape
/// prefix" — exactly four ASCII digits, dash, two ASCII digits,
/// dash, two ASCII digits. Anything else (`...`, `TODO`, `…`) is
/// not a real marker and we skip the line silently. The full
/// validity gate is `parse_iso_date`; this is just a fast-path
/// that lets prose mentions of the marker convention live in
/// documentation without tripping the linter.
fn looks_like_date(s: &str) -> bool {
    if s.len() < 10 {
        return false;
    }
    let bytes = &s.as_bytes()[..10];
    bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

/// Parse `YYYY-MM-DD` (with whatever follows the date — `T...Z`,
/// trailing comma, etc. are tolerated up to position 10). Returns
/// the UNIX-seconds for midnight UTC of that date, or `None` if
/// the date does not parse.
fn parse_iso_date(s: &str) -> Option<u64> {
    if s.len() < 10 {
        return None;
    }
    let prefix = &s[..10];
    let bytes = prefix.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    // Reuse vibe_core's calendar math via the established RFC-3339
    // shape: synthesize `YYYY-MM-DDT00:00:00Z` and feed it back.
    let synthesized = format!("{prefix}T00:00:00Z");
    vibe_core::timestamp::parse_unix_utc(&synthesized)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::parse_iso_date;
    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Finding, Severity, check_project};

    #[test]
    fn review_marker_older_than_threshold_warns() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::create_dir_all(project.path().join("spec/notes")).unwrap();
        // Date 2026-04-01 — about 33 days before fixed_now (2026-05-04).
        // Default threshold is 14d, so it should fire.
        fs::write(
            project.path().join("spec/notes/old-review.md"),
            "<!-- REVIEW: 2026-04-01 fix this later -->\n",
        )
        .unwrap();
        // A fresh marker should NOT fire.
        fs::write(
            project.path().join("spec/notes/new-review.md"),
            "<!-- REVIEW: 2026-05-03 just yesterday -->\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        let aging: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::ReviewAging && f.severity == Severity::Warning)
            .collect();
        assert_eq!(aging.len(), 1, "got: {:?}", report.findings);
        assert!(aging[0].message.contains("2026-04-01"));
        assert_eq!(
            aging[0].path.as_deref().unwrap(),
            Path::new("spec/notes/old-review.md")
        );
        assert_eq!(aging[0].line, Some(1));
    }

    #[test]
    fn review_marker_with_placeholder_is_silently_skipped() {
        // Documentation often references `<!-- REVIEW: ... -->` or
        // `<!-- REVIEW: … -->` as a template. Those are not real
        // markers — they shouldn't trip the linter.
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::create_dir_all(project.path().join("spec/notes")).unwrap();
        fs::write(
            project.path().join("spec/notes/docs.md"),
            "Add a `<!-- REVIEW: ... -->` marker.\n\
             Or `<!-- REVIEW: … -->`.\n\
             Or `<!-- REVIEW: free-form prose -->`.\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        // No ReviewAging findings whatsoever — the placeholder /
        // prose forms are documentation, not work.
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.check != CheckId::ReviewAging),
            "got: {:?}",
            report.findings
        );
    }

    #[test]
    fn review_marker_with_malformed_date_warns() {
        // Date-shaped but invalid (month 13). Real intent is a
        // tracking marker; the operator probably typo'd. Surface
        // a warning so they can fix it.
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::create_dir_all(project.path().join("spec/notes")).unwrap();
        fs::write(
            project.path().join("spec/notes/typo.md"),
            "<!-- REVIEW: 2026-13-01 invalid month -->\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::ReviewAging && f.message.contains("malformed")),
            "got: {:?}",
            report.findings
        );
    }

    #[test]
    fn parse_iso_date_handles_canonical_form() {
        let s = parse_iso_date("2026-04-22").unwrap();
        let echo = vibe_core::timestamp::format_unix_utc(s);
        assert_eq!(echo, "2026-04-22T00:00:00Z");
    }

    #[test]
    fn parse_iso_date_rejects_garbage() {
        assert!(parse_iso_date("not-a-date").is_none());
        assert!(parse_iso_date("2026").is_none());
    }
}
