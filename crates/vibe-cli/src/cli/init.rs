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

/// How a nested package's boot snippet is linked (PROP-009 `link`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum InitLink {
    Static,
    #[default]
    Dynamic,
}

impl InitLink {
    /// The spelling `[boot_snippet].link` takes in the manifest.
    pub fn as_str(self) -> &'static str {
        match self {
            InitLink::Static => "static",
            InitLink::Dynamic => "dynamic",
        }
    }
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
    /// Package kind: flow, feat, stack, tool, mcp, lang, doc, app, remap. Default: tool.
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

    /// Link type of the nested package's boot snippet: static or dynamic.
    /// Default: dynamic. Only `vibe init package <group>/<name>` writes a
    /// boot snippet, so a root declaration and the kinds that carry none
    /// (doc, remap) refuse the flag.
    #[arg(long, value_enum)]
    pub link: Option<InitLink>,

    /// Application authoring choices (only for app packages).
    #[command(flatten)]
    pub app: InitAppArgs,
}

/// The supported authoring paths, lowered to existing installer steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum InitAppWay {
    Recipe,
    Source,
    Prebuilt,
}

/// Container format for an explicitly supplied prebuilt archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum InitAppArchive {
    Zip,
    TarGz,
}

/// Inputs from a creation surface; these declare an installer, never run it.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct InitAppArgs {
    /// Stable application slot id; defaults to the package name.
    #[arg(long = "app-id")]
    pub id: Option<String>,
    /// Target platform: linux, macos or windows; defaults to the detected host.
    #[arg(long = "app-platform")]
    pub platform: Option<String>,
    /// Target architecture token; defaults to the detected host architecture.
    #[arg(long = "app-arch")]
    pub arch: Option<String>,
    /// Recipe (default), verified source plus build recipe, or prebuilt archive.
    #[arg(long = "app-install-way", value_enum)]
    pub way: Option<InitAppWay>,
    /// Input URL: HTTP(S) for source; HTTPS for prebuilt. No download during init.
    #[arg(long = "app-source-url")]
    pub source_url: Option<String>,
    /// Actual 64-hex SHA-256 of the supplied input; never inferred or fabricated.
    #[arg(long = "app-source-sha256")]
    pub source_sha256: Option<String>,
    /// Prebuilt archive container (required for --app-install-way prebuilt).
    #[arg(long = "app-archive-format", value_enum)]
    pub archive: Option<InitAppArchive>,
    /// Optional relative payload directory inside the prebuilt archive.
    #[arg(long = "app-payload-root")]
    pub payload_root: Option<PathBuf>,
    /// Single-line recipe body in the TARGET shell; required for source builds.
    /// It must put the relocatable result in VIBE_OUT. Init only writes it.
    #[arg(long = "app-build-command")]
    pub build_command: Option<String>,
}

impl InitAppArgs {
    pub(crate) fn is_supplied(&self) -> bool {
        self.id.is_some()
            || self.platform.is_some()
            || self.arch.is_some()
            || self.way.is_some()
            || self.source_url.is_some()
            || self.source_sha256.is_some()
            || self.archive.is_some()
            || self.payload_root.is_some()
            || self.build_command.is_some()
    }
}
