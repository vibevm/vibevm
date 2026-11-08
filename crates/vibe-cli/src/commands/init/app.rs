//! Application authoring choices lowered to the incumbent installer declaration.
//! Init writes metadata and recipes; it never fetches inputs or runs a recipe.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use super::helpers::*;
use crate::{
    cli::{InitAppArchive, InitAppArgs, InitAppWay},
    output,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use vibe_core::{
    PackageKind,
    manifest::{
        ApplicationDecl, ArchiveFormatDecl, InstallerOptionDecl, Manifest, ShellInterpreter,
        SourceInputDecl, StepDecl, StepKind,
    },
};

#[derive(Clone, Serialize, Deserialize)]
struct Tables {
    application: ApplicationDecl,
    installer_options: Vec<InstallerOptionDecl>,
}

pub(super) struct Scaffold {
    tables: Tables,
    recipe: Option<(&'static str, String)>,
}

pub(super) fn refused(reason: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-059#declaration: {reason}; fix surface: init --app-* authoring inputs"
    )
}

fn recipe_for(platform: &str) -> (ShellInterpreter, &'static str) {
    if platform == "windows" {
        (ShellInterpreter::WindowsPowershell, "recipes/build.ps1")
    } else {
        (ShellInterpreter::PosixSh, "recipes/build.sh")
    }
}

fn platform_name(platform: &str) -> &str {
    match platform {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        other => other,
    }
}

/// Legacy defaults remain the same; detection belongs to installer selection.
pub(super) fn manifest_tail(name: &str) -> String {
    let platform = vibe_app::detected_platform().unwrap_or(std::env::consts::OS);
    let platform_name = platform_name(platform);
    let (_, script) = recipe_for(platform);
    let interpreter = if platform == "windows" {
        "windows_powershell"
    } else {
        "posix_sh"
    };
    let arch = vibe_app::detected_arch();
    format!(
        "\n[application]\nid = \"{name}\"\n\n[[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"{platform}\"\nplatform_name = \"{platform_name}\"\narch = \"{arch}\"\n\n[[installer_options.steps]]\nid = \"build\"\ntitle = \"Build\"\nkind = \"recipe\"\ninterpreter = \"{interpreter}\"\nscript = \"{script}\"\n"
    )
}

/// Prepare all selected data before either the root or its container is created.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
pub(super) fn prepare(
    args: &InitAppArgs,
    kind: Option<PackageKind>,
    name: &str,
) -> Result<Option<Scaffold>> {
    if kind != Some(PackageKind::App) {
        return if args.is_supplied() {
            Err(refused("application choices require an app package"))
        } else {
            Ok(None)
        };
    }
    if args
        .platform
        .as_deref()
        .is_some_and(|p| !matches!(p, "linux" | "macos" | "windows"))
    {
        return Err(refused(
            "supported scaffold platforms are linux, macos and windows",
        ));
    }
    let mut tables: Tables = toml::from_str(&manifest_tail(name))?;
    if let Some(id) = &args.id {
        tables.application.id = id.clone();
    }
    let option = tables
        .installer_options
        .first_mut()
        .ok_or_else(|| refused("default scaffold option is missing"))?;
    if let Some(platform) = &args.platform {
        option.platform = platform.clone();
        option.platform_name = platform_name(platform).into();
    }
    if let Some(arch) = &args.arch {
        option.arch = arch.clone();
    }
    let (interpreter, script) = recipe_for(&option.platform);
    let way = args.way.unwrap_or(InitAppWay::Recipe);
    if args
        .build_command
        .as_ref()
        .is_some_and(|c| c.trim().is_empty() || c.chars().any(char::is_control))
    {
        return Err(refused(
            "build command must be nonempty and contain no control characters",
        ));
    }
    let recipe = match way {
        InitAppWay::Recipe | InitAppWay::Source => {
            if args.archive.is_some() || args.payload_root.is_some() {
                return Err(refused("archive format and payload root require prebuilt"));
            }
            option.steps.clear();
            if way == InitAppWay::Source {
                let (url, sha256) = source(args)?;
                if args.build_command.is_none() {
                    return Err(refused(
                        "source requires --app-build-command producing a relocatable VIBE_OUT payload",
                    ));
                }
                option.steps.push(StepDecl::new(
                    "sources",
                    "Fetch verified source",
                    StepKind::Sources {
                        sources: vec![SourceInputDecl {
                            name: "source".into(),
                            url: Some(url),
                            path: None,
                            sha256,
                            size: None,
                            extract: None,
                        }],
                    },
                ));
            } else if args.source_url.is_some() || args.source_sha256.is_some() {
                return Err(refused("input URL and hash require source or prebuilt"));
            }
            option.steps.push(StepDecl::new(
                "build",
                "Build",
                StepKind::Recipe {
                    interpreter: Some(interpreter),
                    script: script.into(),
                },
            ));
            Some((
                script,
                recipe_text(name, &option.platform, args.build_command.as_deref()),
            ))
        }
        InitAppWay::Prebuilt => {
            if args.build_command.is_some() {
                return Err(refused("a prebuilt archive does not run a build command"));
            }
            let (source, sha256) = source(args)?;
            let format = match args
                .archive
                .ok_or_else(|| refused("prebuilt requires --app-archive-format zip|tar-gz"))?
            {
                InitAppArchive::Zip => ArchiveFormatDecl::Zip,
                InitAppArchive::TarGz => ArchiveFormatDecl::TarGz,
            };
            option.steps = vec![StepDecl::new(
                "archive",
                "Extract verified archive",
                StepKind::Archive {
                    format,
                    source,
                    sha256,
                    size: None,
                    payload_root: args.payload_root.clone(),
                },
            )];
            None
        }
    };
    Ok(Some(Scaffold { tables, recipe }))
}

fn source(args: &InitAppArgs) -> Result<(String, String)> {
    let url = args
        .source_url
        .as_ref()
        .ok_or_else(|| refused("source/prebuilt requires --app-source-url"))?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(refused("--app-source-url must be an HTTP(S) URL"));
    }
    let hash = args.source_sha256.as_ref().ok_or_else(|| {
        refused("source/prebuilt requires --app-source-sha256; provide the actual digest")
    })?;
    Ok((url.clone(), hash.clone()))
}

fn recipe_text(name: &str, platform: &str, command: Option<&str>) -> String {
    let body = match command {
        None => format!(
            "# The `build` step of the `main` installer option: it runs while the\n# application is installed and writes what gets placed into $VIBE_OUT.\necho \"Building {name}\"\n"
        ),
        Some(command) => format!(
            "# Authored build command. Inputs: VIBE_SOURCES/source (if declared),\n# VIBE_PACKAGE, VIBE_WORK. Write a relocatable payload into VIBE_OUT.\n{command}\n"
        ),
    };
    if platform == "windows" {
        if command.is_some() {
            format!(
                "$ErrorActionPreference = 'Stop'\n{body}\nif ($LASTEXITCODE) {{ exit $LASTEXITCODE }}\n"
            )
        } else {
            body
        }
    } else if command.is_some() {
        format!("#!/bin/sh\nset -eu\n{body}")
    } else {
        format!("#!/bin/sh\n{body}")
    }
}

impl Scaffold {
    pub(super) fn manifest_tail(&self) -> Result<String> {
        Ok(toml::to_string_pretty(&self.tables)?)
    }
    pub(super) fn declare(&self, manifest: &mut Manifest) {
        manifest.application = Some(self.tables.application.clone());
        manifest.installer_options = self.tables.installer_options.clone();
    }
    pub(super) fn check_existing(&self, manifest: &Manifest) -> Result<()> {
        if manifest.application.as_ref() != Some(&self.tables.application)
            || manifest.installer_options != self.tables.installer_options
        {
            return Err(refused(
                "selected application choices differ from the authored manifest; init preserves it",
            ));
        }
        Ok(())
    }
    pub(super) fn create_files(
        &self,
        ctx: &output::Context,
        root: &Path,
        package: &Path,
    ) -> Result<Vec<Outcome>> {
        let mut out = Vec::new();
        if let Some((relative, body)) = &self.recipe {
            let path = package.join(relative);
            let outcome = ensure_file(ctx, root, &path, body, "build recipe")?;
            if outcome.action == Action::Created && relative.ends_with(".sh") {
                mark_executable(&path)?;
            }
            out.push(outcome);
        }
        out.push(ensure_file(
            ctx,
            root,
            &package.join("icons/.gitkeep"),
            "",
            "icon directory",
        )?);
        Ok(out)
    }
}

/// Re-init may restore the known minimal recipe, never guess a custom source build.
pub(super) fn existing(
    args: &InitAppArgs,
    manifest: &Manifest,
    name: &str,
) -> Result<Option<Scaffold>> {
    let kind = manifest.package.as_ref().map(|p| p.kind);
    if args.is_supplied() {
        let scaffold = prepare(args, kind, name)?;
        if let Some(scaffold) = &scaffold {
            scaffold.check_existing(manifest)?;
        }
        return Ok(scaffold);
    }
    if kind != Some(PackageKind::App) {
        return Ok(None);
    }
    let scaffold = prepare(args, kind, name)?;
    Ok(scaffold.filter(|s| s.check_existing(manifest).is_ok()))
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}
#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<()> {
    Ok(())
}
