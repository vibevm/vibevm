//! Root and nested identity creation arguments for `vibe init`.
//!
//! Re-exported by the CLI hub so callers retain `crate::cli::InitArgs`.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

/// The identity declaration written in a new root manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum InitType {
    Project,
    Package,
}

#[derive(Debug, Clone, clap::Args)]
pub struct InitArgs {
    /// Root declaration: project or publishable package (same scaffold).
    #[arg(long = "type", value_enum)]
    pub init_type: Option<InitType>,

    /// Reverse-DNS identity group; required for a package, optional for a project.
    #[arg(long)]
    pub group: Option<String>,
    /// Positional arguments: `[package|group] [pkgref] [path]`.
    ///
    /// Forms:
    ///   vibe init                              — project in CWD (legacy)
    ///   vibe init projectname                  — project in projectname/
    ///   vibe init projectname --type package --group org.example --name demo
    ///   vibe init package org.vibevm.apple/orange [path]  — add package
    ///   vibe init group org.vibevm.apple [path]           — add group
    #[arg(num_args = 0..=3)]
    pub positional: Vec<String>,

    /// Directory to initialize (back-compat with `--path .`).
    /// When positional path is also given, the positional wins.
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Pre-set the active stack name (still requires installation separately).
    #[arg(long)]
    pub stack: Option<String>,

    /// Root project/package name; defaults to the basename of the target directory.
    #[arg(long)]
    pub name: Option<String>,

    /// Override the default registry URL written into `vibe.toml`.
    #[arg(long = "registry-url", conflicts_with = "no_registry")]
    pub registry_url: Option<String>,

    /// Override the default ref (`main`) recorded under `[registry]`.
    #[arg(long = "registry-ref", conflicts_with = "no_registry")]
    pub registry_ref: Option<String>,

    /// Do not write a `[registry]` section into `vibe.toml`.
    #[arg(long = "no-registry")]
    pub no_registry: bool,

    // --- Package creation flags (for `vibe init package` and project+pkg forms) ---
    /// Package kind: flow, feat, stack, tool, mcp, lang, doc, app. Default: tool.
    #[arg(long)]
    pub kind: Option<String>,

    /// Scaffold a translation of the documentation at `<group>/<name>`:
    /// mirrors the source's pages and copies its `[[documents]]`.
    /// Requires `--kind doc`; the language comes from this package's own
    /// name, which a translation spells `<docname>-<lang>`.
    #[arg(long = "translates", value_name = "COORDINATE")]
    pub translates: Option<String>,

    /// Package/project version. Default: 0.1.0 for packages, 0.0.1 for projects.
    #[arg(long)]
    pub version: Option<String>,

    /// Author name (can be repeated). Default: detected from git config.
    #[arg(long = "author")]
    pub authors: Vec<String>,

    /// License. Default: UPL-1.0.
    #[arg(long)]
    pub license: Option<String>,

    /// One-line description.
    #[arg(long)]
    pub description: Option<String>,

    /// Package format: simple or normal. Default: normal.
    #[arg(long)]
    pub format: Option<String>,

    /// Link type for the boot snippet: static or dynamic.
    /// Default: static for project+package, dynamic for `init package`.
    #[arg(long)]
    pub link: Option<String>,
}
