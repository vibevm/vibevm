//! Complete revision-range coverage across a scratch side branch and merge.

use super::tests::Fixture;
use super::*;
use vibe_wire::generated::preview::e1::release_impact::{
    ImplementationSubject, ImplementationSubjectKind,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CONTRIBUTION-SCOPE-PROOF");

#[test]
fn side_branch_commits_and_first_parent_merge_paths_all_require_bindings() {
    let mut fixture = Fixture::new();
    let primary = fixture.git(&["branch", "--show-current"]);
    fixture.git(&["checkout", "--quiet", "-b", "side"]);
    let side = fixture.commit_product("side.rs", b"side source\n");
    fixture.git(&["checkout", "--quiet", &primary]);
    let main = fixture.commit_product("main.rs", b"main source\n");
    fixture.git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "commit.gpgsign=false",
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        "fixture merge",
        "side",
    ]);
    let merge = fixture.git(&["rev-parse", "HEAD"]);
    fixture.after(fixture.snapshot("after"));
    let mut record = fixture.impact(
        "change-branches",
        &main,
        "before",
        "after",
        &["main.rs", "side.rs"],
    );
    record.implementation_subjects.extend([
        ImplementationSubject {
            kind: ImplementationSubjectKind::Commit,
            identity: side.clone(),
        },
        ImplementationSubject {
            kind: ImplementationSubjectKind::Commit,
            identity: merge,
        },
    ]);
    fixture.select(std::slice::from_ref(&record));
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    assert_eq!(
        report
            .checked_claims
            .iter()
            .filter(|claim| claim.starts_with("git-commit-paths:"))
            .count(),
        3
    );
    record
        .implementation_subjects
        .retain(|subject| subject.identity != side);
    fixture.select(&[record]);
    let report = fixture.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Failed),
        "{report:?}"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "commit-uncovered" && finding.subject == side)
    );
}
