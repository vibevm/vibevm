//! Source flags prepare a declaration, but unsupported Git candidates never publish it.
use super::*;
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn git_source_candidate_or_child_input_refusal_preserves_original_authored_file() {
    for child_input in [false, true] {
        let f = Fixture::new(&["base"]);
        let source = f.root.join("new-git-source");
        publish(&source, "new-git", "1.1.0", !child_input);
        let tag = if child_input {
            publish(&f.repo("child"), "child", "1.0.0", false);
            let path = source.join("vibe.toml");
            let mut text = fs::read_to_string(&path)
                .unwrap()
                .replace("version='1.1.0'", "version='1.2.0'");
            text.push_str("\n[requires.packages.'org.vibevm/child']\nversion='1'\nconfiguration={policy='require-selection'}\n");
            Manifest::parse_str(&text).unwrap();
            write(&path, text);
            git(&source, &["add", "vibe.toml"]);
            git(&source, &["commit", "-m", "fixture common child input"]);
            git(&source, &["tag", "v1.2.0"]);
            "v1.2.0"
        } else {
            "v1.1.0"
        };
        let (before, _) = f.snapshots();
        let authored = fs::read(f.project.join("vibe.toml")).unwrap();
        let output = f.run(
            "git-admission",
            &[
                "install",
                "org.vibevm/new-git",
                "--git",
                &format!("file://{}", source.display()),
                "--tag",
                tag,
            ],
        );
        let diagnostic = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = if child_input {
            "does not yet evaluate configuration input"
        } else {
            "does not yet evaluate common configuration"
        };
        assert!(
            !output.status.success() && diagnostic.contains(expected),
            "{diagnostic}"
        );
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
        assert_eq!(
            f.snapshots().0,
            before,
            "source flag must not publish before selected candidate admission"
        );
        assert!(!f.project.join("inplace-hook.log").exists());
    }
}
