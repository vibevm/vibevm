use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const CANONICAL_MD: &str = "# T {#t}\n\n@fact:A body @status:impl/done\n\n";
const LOSSY_MD: &str =
    "# T {#t}\n\n<!-- REVIEW: keep this comment -->\n\n@fact:A body @status:impl/done\n\n";

fn vibe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vibe"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(args)
        .output()
        .expect("run vibe")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("temporary path is Unicode")
}

#[test]
fn clean_markdown_converts_and_removes_the_original() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("clean.md");
    fs::write(&source, CANONICAL_MD).expect("write source");

    let output = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        path_text(temp.path()),
    ]);

    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    assert!(!source.exists());
    assert!(temp.path().join("clean.xml").exists());
    assert!(text(&output.stdout).contains("converted"));
}

#[test]
fn reverse_conversion_restores_canonical_markdown_bytes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("roundtrip.md");
    fs::write(&source, CANONICAL_MD).expect("write source");
    let first = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        path_text(&source),
    ]);
    assert!(first.status.success(), "stderr: {}", text(&first.stderr));

    let xml = temp.path().join("roundtrip.xml");
    let second = vibe(&["refactor", "convert-source", "--to", "md", path_text(&xml)]);

    assert!(second.status.success(), "stderr: {}", text(&second.stderr));
    assert_eq!(
        fs::read(&source).expect("read restored source"),
        CANONICAL_MD.as_bytes()
    );
    assert!(!xml.exists());
}

#[test]
fn non_tty_loss_is_refused_with_the_filename_and_lost_line() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("lossy.md");
    fs::write(&source, LOSSY_MD).expect("write source");

    let output = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        path_text(&source),
    ]);

    assert!(!output.status.success());
    assert!(source.exists());
    assert!(!temp.path().join("lossy.xml").exists());
    let stderr = text(&output.stderr);
    assert!(stderr.contains("lossy.md"), "{stderr}");
    assert!(
        stderr.contains("-<!-- REVIEW: keep this comment -->"),
        "{stderr}"
    );
}

#[test]
fn force_converts_ir_stable_loss() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("lossy.md");
    fs::write(&source, LOSSY_MD).expect("write source");

    let output = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        "--force",
        path_text(&source),
    ]);

    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    assert!(!source.exists());
    assert!(temp.path().join("lossy.xml").exists());
    assert!(text(&output.stdout).contains("lossy-confirmed"));
}

#[test]
fn dry_run_classifies_mixed_corpus_without_writes_and_exits_zero() {
    let temp = tempfile::tempdir().expect("tempdir");
    let clean = temp.path().join("clean.md");
    let lossy = temp.path().join("lossy.md");
    fs::write(&clean, CANONICAL_MD).expect("write clean source");
    fs::write(&lossy, LOSSY_MD).expect("write lossy source");

    let output = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        "--dry-run",
        path_text(temp.path()),
    ]);

    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    assert_eq!(
        fs::read(&clean).expect("read clean source"),
        CANONICAL_MD.as_bytes()
    );
    assert_eq!(
        fs::read(&lossy).expect("read lossy source"),
        LOSSY_MD.as_bytes()
    );
    assert!(!temp.path().join("clean.xml").exists());
    assert!(!temp.path().join("lossy.xml").exists());
    let stdout = text(&output.stdout);
    assert!(stdout.contains("dry-run byte-stable"), "{stdout}");
    assert!(stdout.contains("dry-run ir-stable-loss"), "{stdout}");
}

#[test]
fn recursive_walk_never_converts_vibedeps() {
    let temp = tempfile::tempdir().expect("tempdir");
    let deps = temp.path().join("vibedeps").join("slot");
    fs::create_dir_all(&deps).expect("create vibedeps corpus");
    let source = deps.join("dependency.md");
    fs::write(&source, CANONICAL_MD).expect("write dependency source");

    let output = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        path_text(temp.path()),
    ]);

    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    assert!(source.exists());
    assert!(!deps.join("dependency.xml").exists());
}

#[test]
fn generated_marker_is_skipped_by_walk_but_explicit_file_overrides_it() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("generated.md");
    let generated = format!("<!-- generated by vibe -->\n{CANONICAL_MD}");
    fs::write(&source, &generated).expect("write generated source");

    let walk = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        path_text(temp.path()),
    ]);
    assert!(walk.status.success(), "stderr: {}", text(&walk.stderr));
    assert!(source.exists());
    assert!(!temp.path().join("generated.xml").exists());
    assert!(text(&walk.stdout).contains("skipped-generated"));

    let explicit = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        "--force",
        path_text(&source),
    ]);
    assert!(
        explicit.status.success(),
        "stderr: {}",
        text(&explicit.stderr)
    );
    assert!(!source.exists());
    assert!(temp.path().join("generated.xml").exists());
}

#[test]
fn empty_directory_succeeds_and_missing_path_is_an_honest_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let empty = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        path_text(temp.path()),
    ]);
    assert!(empty.status.success(), "stderr: {}", text(&empty.stderr));
    assert!(text(&empty.stdout).contains("summary converted=0"));

    let missing_path = temp.path().join("missing.md");
    let missing = vibe(&[
        "refactor",
        "convert-source",
        "--to",
        "xml",
        path_text(&missing_path),
    ]);
    assert!(!missing.status.success());
    let stderr = text(&missing.stderr);
    assert!(stderr.contains("missing.md"), "{stderr}");
    assert!(stderr.contains("does not exist"), "{stderr}");
}
