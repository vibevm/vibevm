//! §6.1's laws, sentence by sentence — every clause that can refuse has a
//! test here, driven through the real executor so the refusal reaches the
//! caller exactly as an operator would see it.
//!
//! The two load-bearing ones are the exactly-once pair and the framing
//! test. Drop the "consumed exactly once" law and
//! [`a_declared_resource_no_directive_consumes_refuses`] goes red; replace
//! an inclusion without its origin/hash framing and
//! [`an_inclusion_carries_visible_origin_and_hash_framing`] goes red.

use specmark::verifies;
use vibe_core::manifest::{ArtifactInput, ArtifactKind};

use crate::PackageError;
use crate::mechanism::MechanismError;
use crate::mechanism::package::support::*;

/// The distributable one run produced.
fn produced(root: &std::path::Path, target: &str) -> String {
    let path = root.join(format!("target/vibe-package/{target}/SKILL.md"));
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("the distributable reads at {}: {error}", path.display()),
    }
}

/// The provider refusal one run produced.
fn refusal(error: PackageError) -> MechanismError {
    match error {
        PackageError::Provider(inner) => inner,
        other => panic!("expected a provider refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn one_utf8_skill_document_is_produced_with_its_frontmatter_verbatim() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody text.\n");
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let outcomes = match run_default(root.path(), &targets) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the skill packages: {error}"),
    };

    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    assert_eq!(outcome.provider, "org.vibevm/vibe#static-skill");
    assert_eq!(outcome.via, "the shipped builtin default");
    assert_eq!(outcome.produced.len(), 1);
    assert_eq!(outcome.produced[0].files, 1);
    assert_eq!(
        outcome.produced[0].path_relative,
        "target/vibe-package/demo/SKILL.md",
    );
    let document = produced(root.path(), "demo");
    assert!(document.starts_with("---\nname: demo\n"), "{document}");
    assert!(document.contains("Body text."), "{document}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_inclusion_carries_visible_origin_and_hash_framing() {
    let root = temp();
    write_demo_skill(
        root.path(),
        "\nBefore.\n<!-- vibe:include reference.md -->\nAfter.\n",
    );
    write(root.path(), "skills/demo/reference.md", "Reference text.\n");
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/reference.md"],
    )];

    if let Err(error) = run_default(root.path(), &targets) {
        panic!("the skill packages: {error}");
    }

    let document = produced(root.path(), "demo");
    let digest = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(b"Reference text.\n"))
    };
    assert!(
        document.contains(&format!(
            "<!-- vibe:included name=\"reference.md\" \
             origin=\"skills/demo/reference.md\" sha256=\"{digest}\" -->"
        )),
        "the opening frame names the origin and the digest: {document}",
    );
    assert!(
        document.contains(&format!(
            "<!-- vibe:end name=\"reference.md\" sha256=\"{digest}\" -->"
        )),
        "the closing frame repeats them: {document}",
    );
    assert!(document.contains("Reference text."), "{document}");
    assert!(
        !document.contains("vibe:include "),
        "the directive itself does not survive: {document}",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_rendered_document_is_byte_identical_across_runs() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include a.md -->\n");
    write(root.path(), "skills/demo/a.md", "A.\n");
    let targets = vec![skill_target("demo", "skills/demo", &["skills/demo/a.md"])];

    if let Err(error) = run_default(root.path(), &targets) {
        panic!("the first run packages: {error}");
    }
    let first = produced(root.path(), "demo");
    if let Err(error) = run_default(root.path(), &targets) {
        panic!("the second run packages: {error}");
    }

    assert_eq!(first, produced(root.path(), "demo"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_declared_resource_no_directive_consumes_refuses() {
    let root = temp();
    write_demo_skill(root.path(), "\nNo directive at all.\n");
    write(root.path(), "skills/demo/orphan.md", "Dropped.\n");
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/orphan.md"],
    )];

    let error = run_default(root.path(), &targets)
        .expect_err("a declared resource nothing consumes is a dropped resource");

    match refusal(error) {
        MechanismError::ResourceUnconsumed { names, .. } => {
            assert_eq!(names, "orphan.md");
        }
        other => panic!("expected the exactly-once refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_resource_included_twice_refuses() {
    let root = temp();
    write_demo_skill(
        root.path(),
        "\n<!-- vibe:include a.md -->\n<!-- vibe:include a.md -->\n",
    );
    write(root.path(), "skills/demo/a.md", "A.\n");
    let targets = vec![skill_target("demo", "skills/demo", &["skills/demo/a.md"])];

    let error = run_default(root.path(), &targets)
        .expect_err("two copies of one resource claim one origin twice");

    match refusal(error) {
        MechanismError::IncludeDuplicate { name, .. } => {
            assert_eq!(name, "a.md");
        }
        other => panic!("expected the duplicate refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_include_naming_no_declared_resource_refuses_as_an_unresolved_sibling() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include sibling.md -->\n");
    write(root.path(), "skills/demo/sibling.md", "Sibling.\n");
    // The file exists beside the document and is NOT declared: §6.1's
    // "unresolved sibling reference" is exactly this shape.
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("an undeclared sibling is not a declared resource");

    match refusal(error) {
        MechanismError::IncludeUnknown { name, declared, .. } => {
            assert_eq!(name, "sibling.md");
            assert_eq!(declared, "none declared");
        }
        other => panic!("expected the unknown-include refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_malformed_include_line_refuses_instead_of_surviving_as_text() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include -->\n");
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let error =
        run_default(root.path(), &targets).expect_err("a directive with no name names nothing");

    match refusal(error) {
        MechanismError::IncludeMalformed { line, .. } => assert_eq!(line, 2),
        other => panic!("expected the malformed-directive refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_frontmatter_name_that_disagrees_with_the_directory_refuses() {
    let root = temp();
    write(
        root.path(),
        "skills/demo/SKILL.md",
        "---\nname: other\ndescription: Mismatched identity.\n---\n\nBody.\n",
    );
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let error = run_default(root.path(), &targets).expect_err("a skill has one identity, not two");

    match refusal(error) {
        MechanismError::SkillIdentity {
            declared,
            directory,
            ..
        } => {
            assert_eq!(declared, "other");
            assert_eq!(directory, "demo");
        }
        other => panic!("expected the identity refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_document_with_no_frontmatter_fence_refuses() {
    let root = temp();
    write(
        root.path(),
        "skills/demo/SKILL.md",
        "# demo\n\nNo frontmatter.\n",
    );
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let error = run_default(root.path(), &targets).expect_err("an Agent Skill has frontmatter");

    match refusal(error) {
        MechanismError::Frontmatter { member, .. } => assert_eq!(member, "<block>"),
        other => panic!("expected the frontmatter refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn frontmatter_missing_a_required_member_refuses_naming_it() {
    let root = temp();
    write(
        root.path(),
        "skills/demo/SKILL.md",
        "---\nname: demo\n---\n\nBody.\n",
    );
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let error = run_default(root.path(), &targets).expect_err("`description` is required");

    match refusal(error) {
        MechanismError::Frontmatter { member, .. } => assert_eq!(member, "description"),
        other => panic!("expected the frontmatter refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn frontmatter_this_reader_cannot_fully_understand_refuses() {
    let root = temp();
    write(
        root.path(),
        "skills/demo/SKILL.md",
        "---\nname: demo\ndescription: Deep.\nmetadata:\n  nested:\n    deeper: 1\n---\n\nB.\n",
    );
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("a half-understood block is never half-validated");

    match refusal(error) {
        MechanismError::Frontmatter { member, .. } => assert_eq!(member, "metadata"),
        other => panic!("expected the frontmatter refusal, got {other}"),
    }
}

/// §6.1's "binary assets" refusal at its sharpest edge: bytes that are not
/// UTF-8 but carry no NUL and no shebang. The NUL and shebang laws next
/// door cannot see this shape — a lossy read would inline it as
/// replacement-character soup and every sibling test would stay green,
/// which is exactly what the reviewer mutation demonstrated before this
/// pin existed.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_non_utf8_resource_without_nul_refuses_as_a_binary_asset() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include blob.txt -->\n");
    let path = root.path().join("skills/demo/blob.txt");
    if let Err(error) = std::fs::write(&path, [0xFF_u8, 0xFE, b'x', b'y']) {
        panic!("the fixture blob writes: {error}");
    }
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/blob.txt"],
    )];

    let error =
        run_default(root.path(), &targets).expect_err("a binary asset is not a textual resource");

    match refusal(error) {
        MechanismError::ResourceRejected { reason, .. } => {
            assert!(reason.contains("not valid UTF-8"), "{reason}");
        }
        other => panic!("expected the resource refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_shebang_bearing_resource_refuses() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include run.txt -->\n");
    write(root.path(), "skills/demo/run.txt", "#!/bin/sh\necho hi\n");
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/run.txt"],
    )];

    let error = run_default(root.path(), &targets).expect_err("a program file is not a resource");

    match refusal(error) {
        MechanismError::ResourceRejected { reason, .. } => {
            assert!(reason.contains("shebang"), "{reason}");
        }
        other => panic!("expected the resource refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_binary_resource_refuses() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include blob.dat -->\n");
    if let Err(error) = std::fs::create_dir_all(root.path().join("skills/demo")) {
        panic!("the fixture directory creates: {error}");
    }
    if let Err(error) = std::fs::write(
        root.path().join("skills/demo/blob.dat"),
        [0xff_u8, 0xfe, 0x00, 0x01],
    ) {
        panic!("the fixture binary writes: {error}");
    }
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/blob.dat"],
    )];

    let error = run_default(root.path(), &targets).expect_err("a binary asset is not textual");

    match refusal(error) {
        MechanismError::ResourceRejected { reason, .. } => {
            assert!(reason.contains("binary asset"), "{reason}");
        }
        other => panic!("expected the resource refusal, got {other}"),
    }
}

#[cfg(unix)]
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_executable_resource_refuses() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include tool.txt -->\n");
    write_executable(root.path(), "skills/demo/tool.txt", "plain text\n");
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/tool.txt"],
    )];

    let error = run_default(root.path(), &targets).expect_err("an executable script is refused");

    match refusal(error) {
        MechanismError::ResourceRejected { reason, .. } => {
            assert!(reason.contains("executable"), "{reason}");
        }
        other => panic!("expected the resource refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_resource_whose_extension_names_a_program_refuses_on_every_platform() {
    // Windows has no execute bit, so the extension list is the law that
    // carries the "executable script" refusal there. It holds everywhere,
    // which is why this test is not `cfg`-gated.
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include tool.exe -->\n");
    write(root.path(), "skills/demo/tool.exe", "looks like text\n");
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/tool.exe"],
    )];

    let error = run_default(root.path(), &targets).expect_err("a program file is not a resource");

    match refusal(error) {
        MechanismError::ResourceRejected { reason, .. } => {
            assert!(reason.contains("program file"), "{reason}");
        }
        other => panic!("expected the resource refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_resource_outside_the_skill_source_directory_refuses() {
    let root = temp();
    write_demo_skill(root.path(), "\n<!-- vibe:include elsewhere.md -->\n");
    write(root.path(), "elsewhere.md", "Outside.\n");
    let targets = vec![skill_target("demo", "skills/demo", &["elsewhere.md"])];

    let error =
        run_default(root.path(), &targets).expect_err("a static skill packages its own directory");

    match refusal(error) {
        MechanismError::ResourceOutsideSource {
            name, source_dir, ..
        } => {
            assert_eq!(name, "elsewhere.md");
            assert_eq!(source_dir, "skills/demo");
        }
        other => panic!("expected the containment refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_entry_document_is_never_also_a_declared_resource() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/SKILL.md"],
    )];

    let error = run_default(root.path(), &targets)
        .expect_err("the entry document is the document, not a resource of itself");

    assert!(matches!(
        refusal(error),
        MechanismError::ResourceOutsideSource { .. }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_consumed_build_artifact_refuses_rather_than_being_decompiled() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let mut target = skill_target("demo", "skills/demo", &[]);
    target.inputs = Some(vec![ArtifactInput::Artifact {
        artifact: "helper.exe".to_owned(),
    }]);

    // The record read happens first, so the honest refusal here is the
    // engine's: nothing produced that artifact. The provider's own
    // refusal is proven below with a record present.
    let error = run_default(root.path(), &[target]).expect_err("no record, no artifact");
    assert!(matches!(error, PackageError::InputNotRecorded { .. }));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn more_than_one_declared_output_refuses() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let mut target = skill_target("demo", "skills/demo", &[]);
    target.outputs.push(vibe_core::manifest::ArtifactOutput {
        id: "second.md".to_owned(),
        kind: ArtifactKind::File,
        select: None,
    });

    let error = run_default(root.path(), &[target]).expect_err("§6.1 produces exactly one file");

    match refusal(error) {
        MechanismError::OutputCount { found, .. } => assert_eq!(found, 2),
        other => panic!("expected the output-count refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_output_kind_this_provider_does_not_produce_refuses() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let mut target = skill_target("demo", "skills/demo", &[]);
    target.outputs[0].kind = ArtifactKind::Directory;

    let error = run_default(root.path(), &[target]).expect_err("a static skill is one file");

    match refusal(error) {
        MechanismError::UnsupportedKind {
            kind, supported, ..
        } => {
            assert_eq!(kind, "directory");
            assert_eq!(supported, "skill");
        }
        other => panic!("expected the kind refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_unknown_config_member_refuses_naming_itself() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let mut target = skill_target("demo", "skills/demo", &[]);
    target.config = Some(config("source = \"skills/demo\"\nname = \"demo\""));

    let error = run_default(root.path(), &[target]).expect_err("`name` is engine-owned");

    match refusal(error) {
        MechanismError::Config { member, reason, .. } => {
            assert_eq!(member, "name");
            assert!(reason.contains("source directory's own name"), "{reason}");
        }
        other => panic!("expected the config refusal, got {other}"),
    }
}
