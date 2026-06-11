//! Check 5 — `spec/WAL.md` modification time is less than
//! `wal_max_age_hours` (default 24); older → warning.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use specmark::cell;

use crate::{Check, CheckId, CheckOptions, CheckReport, Finding, Severity};

/// The [`CheckId::WalFreshness`] cell.
#[cell(seam = "Check", variant = "wal-freshness")]
pub struct WalFreshnessCheck;

impl Check for WalFreshnessCheck {
    fn id(&self) -> CheckId {
        CheckId::WalFreshness
    }

    fn run(&self, project_root: &Path, opts: &CheckOptions, report: &mut CheckReport) {
        let now_unix = opts.now_unix_utc.unwrap_or_else(crate::unix_now_utc);
        let max_age_hours = opts.wal_max_age_hours;
        let wal_rel = PathBuf::from("spec/WAL.md");
        let wal = project_root.join(&wal_rel);
        if !wal.exists() {
            // Surfaced once by the WAL well-formedness cell. Don't
            // double-report.
            return;
        }
        let mtime = match fs::metadata(&wal).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(e) => {
                report.warn(
                    CheckId::WalFreshness,
                    Some(wal_rel),
                    None,
                    format!("could not read WAL mtime: {e}"),
                );
                return;
            }
        };
        let mtime_unix = mtime
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if now_unix < mtime_unix {
            // Clock skew — WAL appears to be in the future. Don't fail
            // the check; surface as info so the operator notices.
            report.push(Finding {
                check: CheckId::WalFreshness,
                severity: Severity::Info,
                path: Some(wal_rel),
                line: None,
                message: "WAL mtime is in the future relative to the current clock — possible system-clock skew"
                    .to_string(),
            });
            return;
        }
        let age_secs = now_unix - mtime_unix;
        let age_hours = age_secs / 3600;
        if age_hours > max_age_hours {
            report.warn(
                CheckId::WalFreshness,
                Some(wal_rel),
                None,
                format!(
                    "WAL is {age_hours} hours old (threshold: {max_age_hours}h). Consider an end-session checkpoint."
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::tempdir;

    use crate::test_support::write_minimal_project;
    use crate::{CheckId, CheckOptions, Severity, check_project, unix_now_utc};

    #[test]
    fn wal_older_than_24h_warns() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // Set WAL mtime to ~48h ago.
        let two_days = std::time::Duration::from_secs(48 * 3600);
        let two_days_ago = SystemTime::now() - two_days;
        let wal = project.path().join("spec/WAL.md");
        // `set_modified` is on filetime::set_file_mtime; std doesn't
        // provide it directly. Use a helper via fs::File +
        // set_modified on a fresh stamp via filetime crate? Not
        // available — but we can fall back: set `now_unix_utc` in
        // opts to a moment far past the WAL's actual mtime. That
        // gives us the same age-math without touching mtime.
        let now_far_future = unix_now_utc() + 48 * 3600;
        let opts = CheckOptions {
            now_unix_utc: Some(now_far_future),
            ..Default::default()
        };
        let report = check_project(project.path(), &opts);
        let _ = (two_days_ago, wal);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::WalFreshness && f.severity == Severity::Warning),
            "expected WalFreshness warning; got: {:?}",
            report.findings
        );
    }
}
