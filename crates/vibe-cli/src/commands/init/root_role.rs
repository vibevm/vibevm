//! Pure planning of the root identity, before any scaffold or settings write.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::user_config::UserConfig;
use vibe_core::{
    PackageKind,
    manifest::{Manifest, ProjectSection},
};

use super::{
    prompts::{self, ProjectFields},
    validate_group,
};
use crate::{
    cli::{InitArgs, InitType},
    output,
};

pub(super) struct RootPlan {
    pub role: InitType,
    pub fields: ProjectFields,
    pub manifest: Manifest,
}

pub(super) fn plan(
    ctx: &output::Context,
    args: &InitArgs,
    root: &Path,
    user_config: &UserConfig,
    interactive: bool,
) -> Result<RootPlan> {
    if args.translates.is_some() {
        bail!("`--translates` requires the nested `vibe init package <group>/<name>` form");
    }
    let manifest_path = root.join(Manifest::FILENAME);
    if manifest_path.exists() {
        return existing(args, Manifest::read(&manifest_path)?);
    }
    let default_name = super::helpers::resolve_name(args, root)?;
    let fields = prompts::project_fields_from_args(args, &default_name, user_config);
    let (role, fields, group, kind) = if interactive {
        ctx.suspend_progress(|| {
            let mut dialogue = TerminalIdentity { ctx };
            let (role, fields, group) = collect_identity(args, fields, &mut dialogue)?;
            let fixed_identity = InitArgs {
                name: Some(fields.name.clone()),
                ..args.clone()
            };
            let fields =
                prompts::prompt_fields(&fixed_identity, fields, role == InitType::Package)?;
            let kind = if role == InitType::Package {
                prompts::prompt_kind(args)?
            } else {
                super::package::requested_kind(args)?
            };
            Ok::<_, anyhow::Error>((role, fields, Some(group), kind))
        })?
    } else {
        let role = args.init_type.unwrap_or(InitType::Project);
        let mut fields = fields;
        if role == InitType::Package && args.version.is_none() {
            fields.version = "0.1.0".to_string();
        }
        (
            role,
            fields,
            args.group.clone(),
            super::package::requested_kind(args)?,
        )
    };
    validate_fields(&fields, role == InitType::Package)?;
    if let Some(group) = &group {
        validate_group(group)?;
    }
    let manifest = new_manifest(args, role, group.as_deref(), kind, &fields)?;
    Ok(RootPlan {
        role,
        fields,
        manifest,
    })
}

/// The identity dialogue finishes before metadata and before any writes.
trait IdentityDialogue {
    fn introduction(&mut self);
    fn group(&mut self) -> Result<String>;
    fn name(&mut self, default: &str) -> Result<String>;
    fn role(&mut self) -> Result<InitType>;
}

struct TerminalIdentity<'a> {
    ctx: &'a output::Context,
}

impl IdentityDialogue for TerminalIdentity<'_> {
    fn introduction(&mut self) {
        prompts::identity_introduction(self.ctx);
    }

    fn group(&mut self) -> Result<String> {
        prompts::text("Group (reverse-DNS)", "", false)
    }

    fn name(&mut self, default: &str) -> Result<String> {
        prompts::text("Name within the group", default, false)
    }

    fn role(&mut self) -> Result<InitType> {
        let selected = crate::output::prompts::choose(
            "Root declaration",
            &crate::output::prompts::named(&["project", "package"]),
            0,
        )?;
        Ok(if selected == 0 {
            InitType::Project
        } else {
            InitType::Package
        })
    }
}

fn collect_identity(
    args: &InitArgs,
    mut fields: ProjectFields,
    dialogue: &mut impl IdentityDialogue,
) -> Result<(InitType, ProjectFields, String)> {
    dialogue.introduction();
    let group = match &args.group {
        Some(group) => group.clone(),
        None => dialogue.group()?.trim().to_string(),
    };
    validate_group(&group)?;
    if args.name.is_none() {
        fields.name = dialogue.name(&fields.name)?;
    }
    if fields.name.trim().is_empty() {
        bail!("initialization name must not be empty");
    }
    let role = match args.init_type {
        Some(role) => role,
        None => dialogue.role()?,
    };
    if role == InitType::Package {
        vibe_core::PackageName::parse(&fields.name).context("invalid initialization name")?;
        if args.version.is_none() {
            fields.version = "0.1.0".to_string();
        }
    }
    Ok((role, fields, group))
}

pub(super) fn validate_fields(fields: &ProjectFields, package: bool) -> Result<()> {
    if package {
        vibe_core::PackageName::parse(&fields.name).context("invalid initialization name")?;
    } else if fields.name.trim().is_empty() {
        bail!("project name must not be empty");
    }
    semver::Version::parse(&fields.version).context("invalid initialization version")?;
    if !matches!(fields.format.as_str(), "normal" | "simple") {
        bail!(
            "invalid initialization format `{}`; expected normal or simple",
            fields.format
        );
    }
    Ok(())
}

fn new_manifest(
    args: &InitArgs,
    role: InitType,
    group: Option<&str>,
    kind: PackageKind,
    fields: &ProjectFields,
) -> Result<Manifest> {
    let mut manifest = Manifest {
        active: args
            .stack
            .as_ref()
            .map(|stack| vibe_core::manifest::ActiveSection {
                stack: Some(stack.clone()),
            }),
        registries: super::resolve_registry_sections(args),
        ..Default::default()
    };
    match role {
        InitType::Project => {
            manifest.project = Some(ProjectSection {
                name: fields.name.clone(),
                group: group.map(vibe_core::Group::parse).transpose()?,
                version: fields.version.clone(),
                spec_format: None,
                authors: fields.authors.clone(),
            })
        }
        InitType::Package => {
            let group = group.context("a package root requires `--group <reverse-DNS>`")?;
            if kind == PackageKind::Doc {
                let doc = super::doc::root_manifest(group, &fields.name, fields)?;
                manifest.package = doc.package;
                manifest.documents = doc.documents;
                manifest.i18n = doc.i18n;
            } else {
                let mut table = toml::Table::new();
                for (key, value) in [
                    ("name", fields.name.as_str()),
                    ("group", group),
                    ("kind", kind.as_str()),
                    ("version", fields.version.as_str()),
                    ("license", fields.license.as_str()),
                    ("description", fields.description.as_str()),
                    ("format", fields.format.as_str()),
                ] {
                    table.insert(key.to_string(), toml::Value::String(value.to_string()));
                }
                table.insert("epoch".into(), toml::Value::Integer(1));
                table.insert(
                    "authors".into(),
                    toml::Value::Array(
                        fields
                            .authors
                            .iter()
                            .cloned()
                            .map(toml::Value::String)
                            .collect(),
                    ),
                );
                manifest.package = Some(toml::Value::Table(table).try_into()?);
                super::kinds::declare_in_root(&mut manifest, kind, group, &fields.name)?;
            }
        }
    }
    manifest.validate()?;
    Ok(manifest)
}

fn existing(args: &InitArgs, manifest: Manifest) -> Result<RootPlan> {
    let (role, name, group, version, authors, license, description, format) = if let Some(package) =
        &manifest.package
    {
        (
            InitType::Package,
            package.name.clone(),
            Some(package.group.as_str()),
            package.version.to_string(),
            package.authors.clone(),
            package.license.clone().unwrap_or_default(),
            package.description.clone().unwrap_or_default(),
            match package.format {
                vibe_core::manifest::PackageFormat::Normal => "normal",
                vibe_core::manifest::PackageFormat::Simple => "simple",
            }
            .to_string(),
        )
    } else if let Some(project) = &manifest.project {
        (
            InitType::Project,
            project.name.clone(),
            project.group.as_ref().map(vibe_core::Group::as_str),
            project.version.clone(),
            project.authors.clone(),
            String::new(),
            String::new(),
            "normal".to_string(),
        )
    } else {
        bail!(
            "existing root has no project or package identity; init does not convert workspace roles"
        );
    };
    if args.init_type.is_some_and(|requested| requested != role) {
        bail!("existing root declaration is {role:?}; init does not convert project/package roles");
    }
    let requested_group = args
        .group
        .as_deref()
        .map(|group| {
            validate_group(group)?;
            Ok::<_, anyhow::Error>(vibe_core::Group::parse(group)?)
        })
        .transpose()?;
    if requested_group
        .as_ref()
        .is_some_and(|requested| Some(requested.as_str()) != group)
        || args
            .name
            .as_deref()
            .is_some_and(|requested| requested != name)
    {
        bail!(
            "requested group/name differs from the existing root identity; init preserves authored manifests"
        );
    }
    let requested = prompts::project_fields_from_args(args, &name, &UserConfig::default());
    validate_fields(&requested, role == InitType::Package)?;
    super::package::requested_kind(args)?;
    Ok(RootPlan {
        role,
        fields: ProjectFields {
            name,
            version,
            authors,
            license,
            description,
            format,
        },
        manifest,
    })
}

#[cfg(test)]
mod tests {
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
        let manifest =
            new_manifest(&args(), role, Some(&group), PackageKind::Tool, &selected).unwrap();
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
            PackageKind::Mcp,
            PackageKind::Remap,
        ] {
            let manifest = new_manifest(
                &args(),
                InitType::Package,
                Some("org.example"),
                kind,
                &fields(),
            )
            .unwrap();
            let roundtrip =
                Manifest::parse_str(&toml::to_string_pretty(&manifest).unwrap()).unwrap();
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
    fn row_requiring_kinds_get_the_placeholder_rows_their_validators_ask_for() {
        let mcp = new_manifest(
            &args(),
            InitType::Package,
            Some("org.example"),
            PackageKind::Mcp,
            &fields(),
        )
        .unwrap();
        assert_eq!(mcp.mcp_servers.len(), 1);
        assert_eq!(mcp.mcp_servers[0].binary.as_deref(), Some("sample"));
        assert_eq!(mcp.binaries[0].name, "sample");
        let remap = new_manifest(
            &args(),
            InitType::Package,
            Some("org.example"),
            PackageKind::Remap,
            &fields(),
        )
        .unwrap();
        let [row] = remap.remap.rows.as_slice() else {
            panic!("one remap row, got {:?}", remap.remap.rows);
        };
        assert_eq!(row.prefix.as_deref(), Some("https://example.org/"));
        assert_eq!(row.url.as_deref(), Some("https://mirror.example.org/"));
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
}
