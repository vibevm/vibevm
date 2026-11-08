//! Report admission precedes any pending in-place update and gitignore change.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#provenance");
use super::*;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
fn scoped_inplace_update_refuses_malformed_or_unsafe_report_before_mutations() {
    for kind in ["malformed", "directory", "symlink"] {
        let f = Fixture::new(&["held"]);
        publish(&f.repo("held"), "held", "1.1.0", false);
        // A valid run would append the missing ignore entry as well as advance HEAD.
        write(
            &f.project.join(".gitignore"),
            "# preserve this exact file\n",
        );
        let report = f.project.join("vibe.lock.provenance");
        if report.exists() {
            fs::remove_file(&report).unwrap();
        }
        match kind {
            "malformed" => write(&report, "not valid [ project report\n"),
            "directory" => fs::create_dir(&report).unwrap(),
            "symlink" => {
                let external = f.root.join("outside-report");
                write(&external, "outside bytes\n");
                std::os::unix::fs::symlink(&external, &report).unwrap();
            }
            _ => unreachable!(),
        }
        let slot = snapshot(&f.slot("held"));
        let lock = fs::read(f.project.join("vibe.lock")).unwrap();
        let ignore = fs::read(f.project.join(".gitignore")).unwrap();
        let result = f.run(
            &format!("provenance-{kind}"),
            &["update", "org.vibevm/held"],
        );
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!result.status.success());
        assert!(
            diagnostic.contains("vibe.lock.provenance"),
            "wrong refusal boundary: {diagnostic}"
        );
        assert_eq!(
            snapshot(&f.slot("held")),
            slot,
            "report refusal cannot checkout, clean or rewrite Git metadata"
        );
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), lock);
        assert_eq!(fs::read(f.project.join(".gitignore")).unwrap(), ignore);
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
fn valid_report_control_reaches_scoped_inplace_publication() {
    let f = Fixture::new(&["held"]);
    let expected = publish(&f.repo("held"), "held", "1.1.0", false);
    write(&f.project.join(".gitignore"), "# initial\n");
    write(
        &f.project.join("vibe.lock.provenance"),
        "[[package]]\npackage='org.vibevm/held'\nversion='1.0.0'\nfetched_from='https://project-recorded.invalid/held'\n",
    );
    f.success(&f.run("provenance-valid", &["update", "org.vibevm/held"]));
    assert_eq!(git(&f.slot("held"), &["rev-parse", "HEAD"]), expected);
    assert_eq!(f.version("held"), "1.1.0");
    assert!(
        fs::read_to_string(f.project.join(".gitignore"))
            .unwrap()
            .contains("org.vibevm.held")
    );
    assert!(!f.project.join("vibe.lock.provenance").exists());
}
