//! The queries and the three severity helpers of a [`CheckReport`], out of
//! line from the crate root to keep it within the file budget.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

use std::path::PathBuf;

use specmark::spec;

use crate::{CheckId, CheckReport, Finding, Severity};

impl CheckReport {
    pub fn push(&mut self, f: Finding) {
        self.findings.push(f);
    }
    #[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter")]
    pub fn count(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
    pub(crate) fn err(
        &mut self,
        check: CheckId,
        path: Option<PathBuf>,
        line: Option<usize>,
        msg: impl Into<String>,
    ) {
        self.push(Finding {
            check,
            severity: Severity::Error,
            path,
            line,
            column: None,
            field: None,
            package: None,
            message: msg.into(),
            why: None,
            fix: None,
        });
    }
    pub(crate) fn warn(
        &mut self,
        check: CheckId,
        path: Option<PathBuf>,
        line: Option<usize>,
        msg: impl Into<String>,
    ) {
        self.push(Finding {
            check,
            severity: Severity::Warning,
            path,
            line,
            column: None,
            field: None,
            package: None,
            message: msg.into(),
            why: None,
            fix: None,
        });
    }
    pub(crate) fn info(
        &mut self,
        check: CheckId,
        path: Option<PathBuf>,
        line: Option<usize>,
        msg: impl Into<String>,
    ) {
        self.push(Finding {
            check,
            severity: Severity::Info,
            path,
            line,
            column: None,
            field: None,
            package: None,
            message: msg.into(),
            why: None,
            fix: None,
        });
    }
}
