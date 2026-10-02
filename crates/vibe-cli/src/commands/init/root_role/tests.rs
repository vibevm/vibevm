//! Root identity dialogue and typed-manifest contracts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE");

use super::*;
use clap::Parser;

fn args() -> InitArgs {
    match crate::cli::Cli::try_parse_from(["vibe", "init"])
        .unwrap()
        .command
    {
        crate::cli::Command::Init(args) => args,
        _ => panic!("init parser returned another command"),
    }
}

fn fields() -> ProjectFields {
    ProjectFields {
        name: "sample".into(),
        version: "1.2.3".into(),
        authors: vec!["A \"Writer\"".into()],
        license: "Proprietary".into(),
        description: "Text with \"quotes\"\nand a newline".into(),
        format: "normal".into(),
    }
}

struct FakeIdentity {
    events: Vec<&'static str>,
    group: &'static str,
    name: &'static str,
    role: InitType,
    cancel_at: Option<&'static str>,
}

impl FakeIdentity {
    fn visit(&mut self, event: &'static str) -> Result<()> {
        self.events.push(event);
        if self.cancel_at == Some(event) {
            bail!("cancelled {event}");
        }
        Ok(())
    }
}

impl IdentityDialogue for FakeIdentity {
    fn introduction(&mut self) {
        self.events.push("introduction");
    }
    fn group(&mut self) -> Result<String> {
        self.visit("group")?;
        Ok(self.group.into())
    }
    fn name(&mut self, _default: &str) -> Result<String> {
        self.visit("name")?;
        Ok(self.name.into())
    }
    fn role(&mut self) -> Result<InitType> {
        self.visit("role")?;
        Ok(self.role)
    }
}

fn dialogue() -> FakeIdentity {
    FakeIdentity {
        events: vec![],
        group: "org.example",
        name: "package-name",
        role: InitType::Package,
        cancel_at: None,
    }
}

#[test]
fn identity_introduction_and_namespace_precede_role_and_metadata() {
    let mut dialogue = dialogue();
    let (role, selected, group) = collect_identity(&args(), fields(), &mut dialogue).unwrap();
    assert_eq!(dialogue.events, ["introduction", "group", "name", "role"]);
    assert_eq!(role, InitType::Package);
    assert_eq!(group, "org.example");
    assert_eq!(selected.name, "package-name");
    assert_eq!(selected.version, "0.1.0");
    let manifest = new_manifest(&args(), role, Some(&group), PackageKind::Tool, &selected).unwrap();
    assert_eq!(manifest.require_package().unwrap().name, "package-name");
}

#[test]
fn supplied_identity_skips_questions_and_preserves_values() {
    let mut args = args();
    args.group = Some("org.supplied".into());
    args.name = Some("supplied-name".into());
    args.init_type = Some(InitType::Package);
    args.version = Some("2.3.4".into());
    let fields = prompts::project_fields_from_args(&args, "directory", &UserConfig::default());
    let mut dialogue = dialogue();
    let (_, selected, group) = collect_identity(&args, fields, &mut dialogue).unwrap();
    assert_eq!(dialogue.events, ["introduction"]);
    assert_eq!(group, "org.supplied");
    assert_eq!(selected.name, "supplied-name");
    assert_eq!(selected.version, "2.3.4");
}

#[test]
fn each_supplied_identity_flag_skips_only_its_question() {
    for (flag, expected) in [
        ("group", vec!["introduction", "name", "role"]),
        ("name", vec!["introduction", "group", "role"]),
        ("type", vec!["introduction", "group", "name"]),
    ] {
        let mut args = args();
        match flag {
            "group" => args.group = Some("org.supplied".into()),
            "name" => args.name = Some("supplied-name".into()),
            "type" => args.init_type = Some(InitType::Package),
            _ => unreachable!(),
        }
        let fields = prompts::project_fields_from_args(&args, "directory", &UserConfig::default());
        let mut dialogue = dialogue();
        let (role, selected, group) = collect_identity(&args, fields, &mut dialogue).unwrap();
        assert_eq!(dialogue.events, expected);
        assert_eq!(role, InitType::Package);
        assert_eq!(
            group,
            if flag == "group" {
                "org.supplied"
            } else {
                "org.example"
            }
        );
        assert_eq!(
            selected.name,
            if flag == "name" {
                "supplied-name"
            } else {
                "package-name"
            }
        );
    }
}

#[test]
fn an_explicit_interactive_project_still_requires_a_valid_group() {
    let mut args = args();
    args.init_type = Some(InitType::Project);
    for group in ["", "invalid", "Org.Example", "org.пример"] {
        let mut dialogue = dialogue();
        dialogue.group = group;
        assert!(collect_identity(&args, fields(), &mut dialogue).is_err());
        assert_eq!(dialogue.events, ["introduction", "group"]);
    }
}

#[test]
fn invalid_group_and_cancel_stop_later_identity_questions() {
    let mut invalid = dialogue();
    invalid.group = "invalid";
    assert!(collect_identity(&args(), fields(), &mut invalid).is_err());
    assert_eq!(invalid.events, ["introduction", "group"]);
    for (cancel, expected) in [
        ("group", vec!["introduction", "group"]),
        ("name", vec!["introduction", "group", "name"]),
        ("role", vec!["introduction", "group", "name", "role"]),
    ] {
        let mut cancelled = dialogue();
        cancelled.cancel_at = Some(cancel);
        assert!(collect_identity(&args(), fields(), &mut cancelled).is_err());
        assert_eq!(cancelled.events, expected);
    }
}

#[test]
fn selected_role_keeps_human_project_names_and_enforces_package_grammar() {
    let mut project = dialogue();
    project.role = InitType::Project;
    project.name = "My project";
    assert!(collect_identity(&args(), fields(), &mut project).is_ok());
    for name in ["My project", "-package", "package-", "package--name"] {
        let mut package = dialogue();
        package.name = name;
        assert!(collect_identity(&args(), fields(), &mut package).is_err());
    }
}

#[test]
fn declaration_only_package_kinds_roundtrip_as_root_consumers() {
    for kind in [
        PackageKind::Tool,
        PackageKind::Flow,
        PackageKind::Feat,
        PackageKind::Stack,
        PackageKind::Lang,
        PackageKind::Doc,
        PackageKind::App,
    ] {
        let manifest = new_manifest(
            &args(),
            InitType::Package,
            Some("org.example"),
            kind,
            &fields(),
        )
        .unwrap();
        let roundtrip = Manifest::parse_str(&toml::to_string_pretty(&manifest).unwrap()).unwrap();
        roundtrip.validate().unwrap();
        assert!(roundtrip.project.is_none());
        let package = roundtrip.package.unwrap();
        assert_eq!(package.kind, kind);
        assert_eq!(package.epoch, Some(1));
        assert_eq!(
            package.description.as_deref(),
            Some(fields().description.as_str())
        );
        assert!(roundtrip.boot_snippet.is_none());
    }
}

#[test]
fn mcp_root_requires_server_declarations_before_scaffolding() {
    let error = new_manifest(
        &args(),
        InitType::Package,
        Some("org.example"),
        PackageKind::Mcp,
        &fields(),
    )
    .expect_err("an MCP package without server declarations must fail core validation");
    assert!(error.to_string().contains("mcp_server"));
}

#[test]
fn project_group_and_version_are_preserved_by_typed_factory() {
    let manifest = new_manifest(
        &args(),
        InitType::Project,
        Some("org.example"),
        PackageKind::Tool,
        &fields(),
    )
    .unwrap();
    let roundtrip = Manifest::parse_str(&toml::to_string_pretty(&manifest).unwrap()).unwrap();
    assert!(roundtrip.package.is_none());
    let project = roundtrip.project.unwrap();
    assert_eq!(project.group.unwrap().as_str(), "org.example");
    assert_eq!(project.version, "1.2.3");
}
