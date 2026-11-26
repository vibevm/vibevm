use super::*;

fn finding(severity: Severity) -> Finding {
    Finding {
        check: CheckId::BootDirectory,
        severity,
        path: Some("vibevm/vibespecs/boot/SYSTEM.md".into()),
        line: None,
        column: None,
        field: None,
        package: None,
        message: "git tracks the generated artifact vibevm/vibespecs/boot/SYSTEM.md".into(),
        why: Some(
            "it is derived from committed inputs, so a tracked copy can go stale and be believed"
                .into(),
        ),
        fix: Some("git rm --cached vibevm/vibespecs/boot/SYSTEM.md".into()),
    }
}

#[test]
fn check_human_finding_shows_why_fix_at() {
    assert_eq!(
        finding_block(&finding(Severity::Error)).render_at(100),
        concat!(
            "✗ boot_directory  git tracks the generated artifact vibevm/vibespecs/boot/SYSTEM.md\n",
            "  why   it is derived from committed inputs, so a tracked copy can go stale and be believed\n",
            "  fix   git rm --cached vibevm/vibespecs/boot/SYSTEM.md\n",
            "  at    vibevm/vibespecs/boot/SYSTEM.md"
        )
    );
}

#[test]
fn check_human_counts_line() {
    assert_eq!(counts_line(3, 0, 0), "3 errors · 0 warnings · 0 info");
    assert_eq!(counts_line(1, 1, 2), "1 error · 1 warning · 2 info");
}

#[test]
fn check_human_clean_line() {
    assert_eq!(
        clean_line(Path::new("/work/p2")),
        "✓ clean · every check passed · /work/p2"
    );
}

#[test]
fn check_human_warnings_close_with_a_warning_line() {
    let mut report = CheckReport::default();
    report.findings.push(finding(Severity::Warning));
    assert_eq!(
        closing_line(&report, Path::new("/work/p2")),
        Some("! 0 errors · 1 warning · 0 info".to_owned())
    );
    report.findings.push(finding(Severity::Error));
    assert_eq!(closing_line(&report, Path::new("/work/p2")), None);
}
