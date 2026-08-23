//! Unit tests for [`super`], out-of-line per the file-length budget —
//! included as a plain `#[cfg(test)] mod tests;`, so the module-tree
//! position, and therefore `use super::…`, is unchanged from the inline
//! form.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-049#gate");

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::SnippetPresuppositionCheck;
use crate::{Check, CheckId, CheckOptions, CheckReport, Severity};

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn write_owner(root: &Path, name: &str) {
    write(
        root,
        &format!("packages/org.example/{name}/v1.0.0/vibe.toml"),
        &format!(
            r#"[package]
group = "org.example"
name = "{name}"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"
concepts = ["WAL"]
"#
        ),
    );
    write(
        root,
        &format!("packages/org.example/{name}/v1.0.0/boot/main.md"),
        &format!("# {}\n\nThe WAL discipline.\n", name.to_uppercase()),
    );
}

fn write_consumer(root: &Path, fragment: bool, guarded: bool, body: &str) {
    let fragment_decl = if fragment {
        format!(
            "\n[[boot_snippet.fragment]]\nsource = \"boot/foreign.md\"{}\n",
            if guarded {
                "\nwhen = \"installed:org.example/d\""
            } else {
                ""
            }
        )
    } else {
        String::new()
    };
    write(
        root,
        "packages/org.example/p/v1.0.0/vibe.toml",
        &format!(
            r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"
{fragment_decl}"#
        ),
    );
    write(
        root,
        "packages/org.example/p/v1.0.0/boot/main.md",
        if fragment { "# P\n" } else { body },
    );
    if fragment {
        write(root, "packages/org.example/p/v1.0.0/boot/foreign.md", body);
    }
}

fn run(root: &Path) -> CheckReport {
    let mut report = CheckReport::default();
    SnippetPresuppositionCheck.run(root, &CheckOptions::default(), &mut report);
    report
}

#[test]
fn foreign_concept_in_main_snippet_is_one_warning() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write_consumer(project.path(), false, false, "The WAL is canonical.\n");

    let report = run(project.path());
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let finding = &report.findings[0];
    assert_eq!(finding.check, CheckId::SnippetPresupposition);
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.line, Some(1));
    assert!(finding.message.contains("WAL"), "{finding:?}");
    assert!(finding.message.contains("org.example/d"), "{finding:?}");
}

#[test]
fn foreign_concept_in_matching_installed_fragment_is_clean() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write_consumer(project.path(), true, true, "The WAL is canonical.\n");
    assert!(run(project.path()).findings.is_empty());
}

#[test]
fn empty_concepts_dictionary_is_clean() {
    let project = tempdir().unwrap();
    write_consumer(project.path(), false, false, "The WAL is canonical.\n");
    assert!(run(project.path()).findings.is_empty());
}

/// PROP-049 §4, the dependency exemption: a declared `[requires]` on the
/// concept's owner over a default (public) edge guarantees
/// co-installation, so the bare mention is lawful without a fragment.
#[test]
fn a_declared_dependency_on_the_owner_makes_the_mention_lawful() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/vibe.toml",
        r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"

[requires.packages]
"flow:org.example/d" = "^1.0.0"
"#,
    );
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/boot/main.md",
        "The WAL is canonical here too.\n",
    );
    assert!(run(project.path()).findings.is_empty());
}

/// PROP-050 ##CONCEPTS-GATE-SOFTENED, lawful homonymy: a package that
/// declares the lexeme in its own `[boot_snippet].concepts` owns the
/// word in its own world, despite the foreign owner.
#[test]
fn own_concept_declaration_makes_the_homonym_lawful() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/vibe.toml",
        r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"
concepts = ["WAL"]
"#,
    );
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/boot/main.md",
        "Our WAL is a different discipline.\n",
    );
    assert!(run(project.path()).findings.is_empty());
}

/// PROP-050 ##CONCEPTS-GATE-SOFTENED, owner-dedup: an unexplained use
/// of a multi-owner lexeme warns once, naming every owner.
#[test]
fn two_owners_one_warning_naming_both() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write_owner(project.path(), "e");
    write_consumer(project.path(), false, false, "The WAL is canonical.\n");

    let report = run(project.path());
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let finding = &report.findings[0];
    assert_eq!(finding.severity, Severity::Warning);
    assert!(
        finding
            .message
            .contains("package(s) `org.example/d`, `org.example/e`"),
        "{finding:?}"
    );
}

/// PROP-050 ##CONCEPTS-GATE-SOFTENED: a relation to ANY ONE of several
/// owners legitimises the mention — here a `[requires]` edge to one.
#[test]
fn a_dependency_on_one_of_two_owners_silences_the_warning() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write_owner(project.path(), "e");
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/vibe.toml",
        r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"

[requires.packages]
"flow:org.example/d" = "^1.0.0"
"#,
    );
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/boot/main.md",
        "The WAL is canonical here too.\n",
    );
    assert!(run(project.path()).findings.is_empty());
}

/// PROP-050 ##DEPS-EXEMPTION-NARROWS: a `private` edge reaches no
/// consumer, so a declared dependency over it no longer exempts the
/// mention — the warning stays.
#[test]
fn a_private_edge_on_the_owner_no_longer_exempts() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/vibe.toml",
        r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"

[requires.packages]
"flow:org.example/d" = { version = "^1.0.0", access = "private" }
"#,
    );
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/boot/main.md",
        "The WAL is canonical here too.\n",
    );
    let report = run(project.path());
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
}

/// PROP-050 ##DEPS-EXEMPTION-NARROWS: a `friends-only` edge seeps
/// toward consumers, so it still justifies the dependency exemption.
#[test]
fn a_friends_only_edge_on_the_owner_exempts() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/vibe.toml",
        r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"

[requires.packages]
"flow:org.example/d" = { version = "^1.0.0", access = "friends-only" }
"#,
    );
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/boot/main.md",
        "The WAL is canonical here too.\n",
    );
    assert!(run(project.path()).findings.is_empty());
}

/// PROP-050 ##CONCEPTS-GATE-SOFTENED (d): the scanned package's own
/// `[visibility].ignore-concept-warnings` mutes the named concept.
#[test]
fn own_ignore_list_mutes_the_named_concept() {
    let project = tempdir().unwrap();
    write_owner(project.path(), "d");
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/vibe.toml",
        r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"

[visibility]
ignore-concept-warnings = ["WAL"]
"#,
    );
    write(
        project.path(),
        "packages/org.example/p/v1.0.0/boot/main.md",
        "The WAL is canonical here too.\n",
    );
    assert!(run(project.path()).findings.is_empty());
}

/// PROP-050 ##CONCEPTS-GATE-SOFTENED (d): the project root's
/// `[visibility].ignore-concept-warnings` mutes the named concept for
/// every package in the tree.
#[test]
fn root_ignore_list_mutes_across_the_tree() {
    let project = tempdir().unwrap();
    write(
        project.path(),
        "vibe.toml",
        r#"[project]
name = "demo"
version = "0.1.0"

[visibility]
ignore-concept-warnings = ["WAL"]
"#,
    );
    write_owner(project.path(), "d");
    write_consumer(project.path(), false, false, "The WAL is canonical.\n");
    assert!(run(project.path()).findings.is_empty());
}

/// PROP-050 ##CONCEPTS-GATE-SOFTENED (d): an ignore entry naming a
/// different lexeme mutes nothing — the warning stays.
#[test]
fn an_unrelated_ignore_entry_does_not_mute() {
    let project = tempdir().unwrap();
    write(
        project.path(),
        "vibe.toml",
        r#"[project]
name = "demo"
version = "0.1.0"

[visibility]
ignore-concept-warnings = ["SCIM"]
"#,
    );
    write_owner(project.path(), "d");
    write_consumer(project.path(), false, false, "The WAL is canonical.\n");
    let report = run(project.path());
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
}
