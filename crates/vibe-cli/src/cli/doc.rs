//! Argument structs for `vibe doc` — the documentation surface over the
//! `vibe-doc` library (PROP-057 `##PIPE-LIBRARY`). Split from the `cli` hub
//! along command-family lines; the hub re-exports them, so
//! `crate::cli::DocArgs` is the address.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

/// `vibe doc` — work with a documentation package.
#[derive(Debug, clap::Args)]
pub struct DocArgs {
    #[command(subcommand)]
    pub command: DocCommand,
}

/// The `vibe doc` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum DocCommand {
    /// Render a documentation package: every page in the projection you
    /// ask for, the page manifest, the four `llms` tiers and the card's
    /// images, at the addresses the site mounts them under.
    Build(DocBuildArgs),

    /// Check a documentation package against the product it documents:
    /// run every documented example and compare its output exactly,
    /// rebuild every `derived` block and compare it with the last build,
    /// resolve every `rule` citation against the current specs, check a
    /// translation against the documentation it adapts, measure how much
    /// of what the specifications promised is told, and judge the card's
    /// images.
    Check(DocCheckArgs),

    /// Print what a machine reads about a documentation package: the
    /// page manifest, or one tier of `llms.txt`.
    Manifest(DocManifestArgs),

    /// Read the documentation locally: an HTTP server on the loopback
    /// that renders a page on every request. Until the reader's shell
    /// ships it serves bare islands — the same content HTML the public
    /// site glues into its own frame.
    Serve(DocServeArgs),
}

/// `vibe doc build` — render the package.
#[derive(Debug, clap::Args)]
pub struct DocBuildArgs {
    /// The documentation package to render. Defaults to the current
    /// directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Where the rendered tree goes. It is written in the site's own
    /// address map, so the directory can be served as-is.
    #[arg(long, default_value = ".vibe/doc", value_name = "DIR")]
    pub out: PathBuf,

    /// Which projection each page is written in: the island (`html`),
    /// the Markdown an agent reads, or the dialect XML that keeps a
    /// citation's address instead of its text.
    #[arg(long, default_value = "html", value_parser = ["html", "md", "xml"])]
    pub format: String,

    /// The path the result will be served under. The site mounts at
    /// `/doc/`; a reader elsewhere passes its own, and the links inside
    /// the pages come out right.
    #[arg(long, default_value = vibe_doc::content::SITE_BASE, value_name = "PATH")]
    pub base: String,

    /// The language you expect the package to be in. One documentation
    /// package is one language, so this states which one you want and
    /// refuses a package written in another, naming the one that holds
    /// it.
    #[arg(long, value_name = "TAG")]
    pub lang: Option<String>,

    /// Do not run the product to generate `derived` blocks. A build from
    /// a warmed store on a machine that compiled nothing still produces
    /// every page; the generated blocks are marked as the gaps they are.
    #[arg(long)]
    pub no_derived: bool,

    /// The `vibe` binary the `derived` generators run. Defaults to the
    /// running one.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,
}

/// `vibe doc manifest` — what a machine reads about the package.
#[derive(Debug, clap::Args)]
pub struct DocManifestArgs {
    /// The documentation package. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Print the page manifest as JSON — the same document the site
    /// serves at `/doc/manifest.json`.
    #[arg(long)]
    pub json: bool,

    /// Print one `llms` tier instead: the index, or the corpus cut to a
    /// token budget.
    #[arg(long, value_name = "TIER", value_parser = ["index", "small", "medium", "full"])]
    pub llms: Option<String>,

    /// The base the links are built on.
    #[arg(long, default_value = vibe_doc::content::SITE_BASE, value_name = "PATH")]
    pub base: String,

    /// The language you expect the package to be in.
    #[arg(long, value_name = "TAG")]
    pub lang: Option<String>,
}

/// `vibe doc serve` — the local reader.
#[derive(Debug, clap::Args)]
pub struct DocServeArgs {
    /// The documentation package to read. Defaults to the current
    /// directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// The port on the loopback. The host is not a setting: the reader
    /// binds `127.0.0.1` and nothing else.
    #[arg(long, default_value_t = vibe_doc_server::DEFAULT_PORT, value_name = "PORT")]
    pub port: u16,

    /// The path the documentation is mounted at.
    #[arg(long, default_value = vibe_doc::content::SITE_BASE, value_name = "PATH")]
    pub base: String,

    /// The language you expect the package to be in.
    #[arg(long, value_name = "TAG")]
    pub lang: Option<String>,

    /// The origin allowed to frame the reader, for an editor's webview.
    /// Absent means no origin at all, which is right for a reader
    /// started from a terminal.
    #[arg(long, value_name = "ORIGIN")]
    pub frame_ancestor: Option<String>,

    /// Do not run the product to generate `derived` blocks at start-up.
    #[arg(long)]
    pub no_derived: bool,

    /// The `vibe` binary the `derived` generators run. Defaults to the
    /// running one.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,
}

/// `vibe doc check` — the checks that keep a manual from lying.
#[derive(Debug, clap::Args)]
pub struct DocCheckArgs {
    /// Run every `<example>` against the built binary in a sandbox and
    /// compare stdout, stderr and the exit code exactly, after the
    /// normalisation each fixture declares.
    #[arg(long)]
    pub examples: bool,

    /// Rebuild every `<derived>` block from the product and report what
    /// changed since the last build.
    #[arg(long)]
    pub derived: bool,

    /// Resolve every `<rule>` citation against the specifications that
    /// exist now. One question and no other: does the anchor exist.
    #[arg(long)]
    pub citations: bool,

    /// Check a translation against the documentation it adapts: the same
    /// pages, the same anchors, examples borrowed rather than authored.
    /// Structure only — there is no «how far behind» here, because that
    /// needs a history this product does not keep.
    #[arg(long)]
    pub translations: bool,

    /// Require every documentation obligation of the specifications — a
    /// fact marked `actionstage="doc"` naming an audience — to be cited
    /// by a page written for that same audience.
    #[arg(long)]
    pub coverage: bool,

    /// With `--coverage`, the percentage that counts as a pass. The
    /// standing bar is everything told; a lower one is for the
    /// intermediate runs of a campaign that is still writing the pages.
    #[arg(long, default_value_t = vibe_doc::coverage::FULL_COVERAGE, value_name = "PERCENT")]
    pub min: u8,

    /// Check the card's images: that each declared file is there, is the
    /// format its bytes say it is, and fits the shape and the byte
    /// ceiling its role is shown at. A role that declares nothing is not
    /// a role without a picture — the placeholder is generated.
    #[arg(long)]
    pub media: bool,

    /// Record a capture on the page when the example has no golden yet.
    /// Never replaces a golden that already holds text — see `--force`.
    #[arg(long)]
    pub accept: bool,

    /// With `--accept`, also replace a golden that already holds text.
    /// A separate flag on purpose: re-blessing a divergence is a decision
    /// somebody takes, not a side effect of running a check.
    #[arg(long)]
    pub force: bool,

    /// Run only the examples whose `<page>#<id>` contains this text.
    #[arg(long, value_name = "TEXT")]
    pub only: Option<String>,

    /// The documentation package to check. Defaults to the current
    /// directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// The `vibe` binary the examples run. Defaults to the running one,
    /// so a check always exercises the build that is asking.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,

    /// Where sandboxes are built. Keep it short: a deep root plus a
    /// materialised dependency tree overflows the Windows path limit.
    #[arg(long, value_name = "PATH")]
    pub sandbox: Option<PathBuf>,

    /// Seconds one documented command may take before it is killed.
    #[arg(long, default_value_t = 300, value_name = "SECONDS")]
    pub timeout: u64,
}
