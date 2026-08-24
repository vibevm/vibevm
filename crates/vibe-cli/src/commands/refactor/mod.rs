//! `vibe refactor` — meaning-preserving source rewrites (PROP-051).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-051#convert-source");

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use vibe_specdoc::Direction;

use crate::output;

mod convert_source;

use convert_source::{ConversionTarget, run_conversion};

/// Arguments for the refactoring command family.
#[derive(Debug, Args)]
pub struct RefactorArgs {
    #[command(subcommand)]
    command: RefactorCommand,
}

#[derive(Debug, Subcommand)]
// The shared `Convert` prefix is the intentional user-facing clap namespace.
#[allow(clippy::enum_variant_names)]
enum RefactorCommand {
    /// Convert selected authored spec sources through the vibe-specdoc pivot.
    #[command(visible_alias = "convert-src")]
    ConvertSource(ConvertSourceArgs),

    /// Convert every authored spec source below one or more package roots.
    ConvertPackageSrc(ConvertPackageSrcArgs),

    /// Convert only the spec/ tree of a package (the current project by default).
    ConvertSpecSrc(ConvertSpecSrcArgs),
}

/// Shared conversion flags for all three source-conversion verbs.
#[derive(Debug, Args)]
struct ConversionArgs {
    /// Source format filter (`md` is an alias for `markdown`).
    #[arg(long, value_enum)]
    from: Option<SourceFormat>,

    /// Target source format (`md` is an alias for `markdown`).
    #[arg(long, value_enum)]
    to: SourceFormat,

    /// Permit IR-stable byte/content losses. Never permits IR divergence.
    #[arg(long)]
    force: bool,

    /// Classify and report without writing; always exits successfully.
    #[arg(long)]
    dry_run: bool,
}

/// Arguments for `vibe refactor convert-source`.
#[derive(Debug, Args)]
struct ConvertSourceArgs {
    #[command(flatten)]
    conversion: ConversionArgs,

    /// Files or directories to convert. Defaults to the current directory.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
}

/// Arguments for `vibe refactor convert-package-src`.
#[derive(Debug, Args)]
struct ConvertPackageSrcArgs {
    #[command(flatten)]
    conversion: ConversionArgs,

    /// Package roots, each containing a vibe.toml.
    #[arg(value_name = "PACKAGE_ROOT", required = true)]
    package_roots: Vec<PathBuf>,
}

/// Arguments for `vibe refactor convert-spec-src`.
#[derive(Debug, Args)]
struct ConvertSpecSrcArgs {
    #[command(flatten)]
    conversion: ConversionArgs,

    /// Package root containing vibe.toml. Defaults to the nearest current project.
    #[arg(value_name = "PACKAGE_ROOT")]
    package_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum SourceFormat {
    Xml,
    #[value(alias = "md")]
    Markdown,
}

impl SourceFormat {
    fn counterpart(self) -> Self {
        match self {
            Self::Xml => Self::Markdown,
            Self::Markdown => Self::Xml,
        }
    }

    pub(super) fn direction(self) -> Direction {
        match self {
            Self::Xml => Direction::ToXml,
            Self::Markdown => Direction::ToMarkdown,
        }
    }

    pub(super) fn extension(self) -> &'static str {
        match self {
            Self::Xml => "xml",
            Self::Markdown => "md",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResolvedConversion {
    pub(super) from: SourceFormat,
    pub(super) to: SourceFormat,
    pub(super) force: bool,
    pub(super) dry_run: bool,
}

impl ConversionArgs {
    fn resolve(self) -> Result<ResolvedConversion> {
        let (from, to) = resolve_formats(self.from, self.to)?;
        Ok(ResolvedConversion {
            from,
            to,
            force: self.force,
            dry_run: self.dry_run,
        })
    }
}

fn resolve_formats(
    from: Option<SourceFormat>,
    to: SourceFormat,
) -> Result<(SourceFormat, SourceFormat)> {
    let from = from.unwrap_or_else(|| to.counterpart());
    if from == to {
        bail!("source and target formats must differ (`--from` equals `--to`)")
    }
    Ok((from, to))
}

/// Run one refactoring-family command.
pub fn run(ctx: &output::Context, args: RefactorArgs) -> Result<()> {
    match args.command {
        RefactorCommand::ConvertSource(args) => run_convert_source(ctx, args),
        RefactorCommand::ConvertPackageSrc(args) => run_convert_package_src(ctx, args),
        RefactorCommand::ConvertSpecSrc(args) => run_convert_spec_src(ctx, args),
    }
}

fn run_convert_source(ctx: &output::Context, args: ConvertSourceArgs) -> Result<()> {
    let conversion = args.conversion.resolve()?;
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths
    };
    let targets = paths.into_iter().map(ConversionTarget::Path).collect();
    run_conversion(ctx, targets, conversion, "convert-source")
}

fn run_convert_package_src(ctx: &output::Context, args: ConvertPackageSrcArgs) -> Result<()> {
    let conversion = args.conversion.resolve()?;
    let targets = args
        .package_roots
        .into_iter()
        .flat_map(resolve_package_targets)
        .collect();
    run_conversion(ctx, targets, conversion, "convert-package-src")
}

fn run_convert_spec_src(ctx: &output::Context, args: ConvertSpecSrcArgs) -> Result<()> {
    let conversion = args.conversion.resolve()?;
    let target = match args.package_root {
        Some(root) => resolve_spec_target(root),
        None => spec_target(find_current_project_root()?),
    };
    run_conversion(ctx, vec![target], conversion, "convert-spec-src")
}

/// The owner's perimeter ruling (2026-08-24): a package conversion covers
/// the package's SPEC HOMES — the live specs root and a nested packages
/// root — never the whole tree («переводить спецификации из директории
/// спецификаций, а не все подряд файлы»). House-specific root working docs
/// (one project's WAL/checkpoint family) are NOT presumed: the verb serves
/// every project, and such names may mean anything elsewhere — a house
/// converts its own extras with explicit `convert-source` paths.
///
/// The home names come from the layout module in `current_*` form
/// (PROP-052 L5): after the R4 flip the homes are the new-layout roots,
/// and the old roots stop being spec homes with no edit here.
fn resolve_package_targets(path: PathBuf) -> Vec<ConversionTarget> {
    let root = match super::resolve_project_root(&path) {
        Ok(root) => root,
        Err(error) => {
            return vec![ConversionTarget::Refused {
                path,
                reason: format!("{error:#}"),
            }];
        }
    };
    let mut targets = Vec::new();
    for home in [
        root.join(vibe_core::layout::current_specs_root()),
        root.join(vibe_core::layout::current_packages_root()),
    ] {
        if home.is_dir() {
            targets.push(ConversionTarget::Path(home));
        }
    }
    if targets.is_empty() {
        let specs = vibe_core::machine_json_path(&vibe_core::layout::current_specs_root());
        let packs = vibe_core::machine_json_path(&vibe_core::layout::current_packages_root());
        targets.push(ConversionTarget::Refused {
            path: root,
            reason: format!("package has no spec homes (`{specs}/` or `{packs}/`)"),
        });
    }
    targets
}

fn resolve_spec_target(path: PathBuf) -> ConversionTarget {
    match super::resolve_project_root(&path) {
        Ok(root) => spec_target(root),
        Err(error) => ConversionTarget::Refused {
            path,
            reason: format!("{error:#}"),
        },
    }
}

fn spec_target(root: PathBuf) -> ConversionTarget {
    let spec = root.join(vibe_core::layout::current_specs_root());
    if spec.is_dir() {
        ConversionTarget::Path(spec)
    } else {
        ConversionTarget::Refused {
            path: spec,
            reason: format!(
                "package has no `{}/` directory",
                vibe_core::machine_json_path(&vibe_core::layout::current_specs_root())
            ),
        }
    }
}

fn find_current_project_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("reading current directory")?;
    let Some(root) = cwd
        .ancestors()
        .find(|candidate| candidate.join("vibe.toml").is_file())
    else {
        bail!(
            "current project root not found: no `vibe.toml` at or above `{}`",
            cwd.display()
        )
    };
    super::resolve_project_root(root)
}

#[cfg(test)]
mod tests {
    use super::{SourceFormat, resolve_formats};

    #[test]
    fn equal_source_and_target_formats_are_rejected() {
        let error = resolve_formats(Some(SourceFormat::Xml), SourceFormat::Xml)
            .expect_err("equal formats must fail");
        assert!(error.to_string().contains("must differ"));
    }

    #[test]
    fn source_format_defaults_to_target_counterpart() {
        assert_eq!(
            resolve_formats(None, SourceFormat::Xml).expect("resolve xml target"),
            (SourceFormat::Markdown, SourceFormat::Xml)
        );
        assert_eq!(
            resolve_formats(None, SourceFormat::Markdown).expect("resolve markdown target"),
            (SourceFormat::Xml, SourceFormat::Markdown)
        );
    }
}
