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

    /// Render the whole site: every version the configured registry
    /// currently publishes, and the host from the checkout on disk. This
    /// is the command the renderer container runs — it reads two
    /// sources, compares them with what it last rendered, and rebuilds
    /// only what moved.
    BuildSite(DocBuildSiteArgs),

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
    /// that renders a page on every request and glues it into the
    /// reader's shell — the same shell the public site wears, built into
    /// this binary or placed in the machine store, and the bare one when
    /// neither is there.
    Serve(DocServeArgs),

    /// The reader's shell: what this binary carries, and how to get the
    /// real one when it carries the bare fallback.
    Shell(DocShellArgs),

    /// Record what the product's surface looks like on one declared
    /// version: the command tree with its flags, the keys the manifest
    /// and the lock file accept, the members of every published schema,
    /// the text of every documentation obligation, and the format
    /// registry. There is no hash, no build date and no state identifier
    /// in it — a version is a behavioural contract, and the number the
    /// owner declares is the only thing this project compares versions
    /// by.
    Surface(DocSurfaceArgs),

    /// Compare two recorded surfaces and print the pages to update, each
    /// with the reason that reached it: a rule the page cites, a command
    /// or a schema the page derives its text from, or a promise made to
    /// an audience nobody tells. A change no page answers to is reported
    /// as a page that does not exist yet.
    Diff(DocDiffArgs),

    /// The maintenance queue by the CURRENT state of the package and the
    /// product: obligations nobody tells, citations that no longer
    /// resolve, adaptations that do not mirror, pages owed a reading
    /// aloud, documentation debt and what the style linter found. It
    /// prints the numbers and returns success whatever they say — no
    /// technical gate binds a release of the product to its
    /// documentation, so this measures rather than stops.
    Todo(DocTodoArgs),
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

/// `vibe doc build-site` — the registry builder.
#[derive(Debug, clap::Args)]
pub struct DocBuildSiteArgs {
    /// The site's configuration: the two sources it reads, in the form a
    /// project names a `[[registry]]`, and the one table that says which
    /// domain the render is for.
    #[arg(long, default_value = vibe_doc::site::config::FILENAME, value_name = "FILE")]
    pub config: PathBuf,

    /// Where the rendered site goes. The builder keeps its own state
    /// under `.vibe-site/` inside it, so pointing a second run at the
    /// same directory is what makes it rebuild only what moved.
    #[arg(long, default_value = ".vibe/site", value_name = "DIR")]
    pub out: PathBuf,

    /// Read the sources and print what would be rebuilt, writing
    /// nothing.
    #[arg(long)]
    pub dry_run: bool,

    /// The site package that turns the rendered trees into the domain.
    /// Defaults to the one in the host's checkout, which is where a
    /// renderer has one.
    #[arg(long, value_name = "DIR")]
    pub web: Option<PathBuf>,

    /// Render the documentation trees and stop. For a machine with no
    /// Node, and for looking at what the pipeline produced before the
    /// site is built on it.
    #[arg(long)]
    pub no_web: bool,
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

    /// Print what shell this reader would serve — where it came from, the
    /// package it was built from, and the three digests that have to
    /// agree — as JSON, and start no server.
    #[arg(long)]
    pub print_shell: bool,

    /// Serve the bare shell even when a real one is available. For
    /// reading a page the way a machine with no Node sees it.
    #[arg(long)]
    pub bare_shell: bool,

    /// The `vibe` binary the `derived` generators run. Defaults to the
    /// running one.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,
}

/// `vibe doc shell` — the reader's shell.
#[derive(Debug, clap::Args)]
pub struct DocShellArgs {
    #[command(subcommand)]
    pub command: Option<DocShellCommand>,
}

/// The `vibe doc shell` subcommands. None means `status`.
#[derive(Debug, clap::Subcommand)]
pub enum DocShellCommand {
    /// What shell this binary carries, and whether it is the one the
    /// build pinned: the digest measured from the bytes now, the digest
    /// the shell's own index records, the digest compiled in, and the
    /// pin. Non-zero exit when they disagree.
    Status(DocShellStatusArgs),

    /// Download the shell this version of `vibe` was built with and place
    /// it in the machine store. Asks first, every time, and never runs by
    /// itself: a reader that reached the network on its own would have
    /// broken the one promise the local mode makes.
    Install(DocShellInstallArgs),
}

/// `vibe doc shell status`.
#[derive(Debug, clap::Args)]
pub struct DocShellStatusArgs {
    /// Print the report as JSON.
    #[arg(long)]
    pub json: bool,
}

/// `vibe doc shell install`.
#[derive(Debug, clap::Args)]
pub struct DocShellInstallArgs {
    /// Take the download as approved. The flag is the consent — there is
    /// no setting that makes it permanent.
    #[arg(long)]
    pub assume_yes: bool,

    /// Where the release assets are read from. The default is the
    /// project's own releases; a mirror or a local directory is for a
    /// test and for an air-gapped copy.
    #[arg(long, value_name = "URL")]
    pub from: Option<String>,
}

/// `vibe doc surface` — the surface of one declared version.
#[derive(Debug, clap::Args)]
pub struct DocSurfaceArgs {
    /// The version number this snapshot is recorded under. It is your
    /// word and nothing reads it back: the product cannot tell one amend
    /// of a version from another, so the number is an input.
    #[arg(long, value_name = "VERSION")]
    pub record: String,

    /// The documentation package the snapshot is written into. Defaults
    /// to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Write the snapshot here instead of the package's own
    /// `maintenance/surface/`.
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// The `vibe` binary whose surface is recorded. Defaults to the
    /// running one, so a snapshot describes the build that took it.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,

    /// Seconds one `--help` may take before it is killed.
    #[arg(long, default_value_t = 300, value_name = "SECONDS")]
    pub timeout: u64,
}

/// `vibe doc diff` — two surfaces, and the pages between them.
#[derive(Debug, clap::Args)]
pub struct DocDiffArgs {
    /// The older version, by the number its snapshot was recorded under.
    #[arg(value_name = "OLD")]
    pub from: String,

    /// The newer version. The word `now` means the product this command
    /// is, compared without recording anything — a hint to a full
    /// reconciliation about which pages to re-read first. It is off by
    /// default because it is not a version: see `--allow-now`.
    #[arg(value_name = "NEW")]
    pub to: String,

    /// Permit `now` as the newer side. Without it, `now` is refused:
    /// inside a version the product changes invisibly by design, so a
    /// comparison against the working tree answers a question the project
    /// does not otherwise ask, and the flag is where that decision is
    /// visible.
    #[arg(long)]
    pub allow_now: bool,

    /// How the answer is printed: for a person, or for a machine.
    #[arg(long, default_value = "md", value_parser = ["md", "json"])]
    pub format: String,

    /// The documentation package whose pages the changes are placed in.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// The `vibe` binary `now` is read from.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,

    /// Seconds one `--help` may take before it is killed.
    #[arg(long, default_value_t = 300, value_name = "SECONDS")]
    pub timeout: u64,
}

/// `vibe doc todo` — the maintenance queue.
#[derive(Debug, clap::Args)]
pub struct DocTodoArgs {
    /// The documentation package. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// How the queue is printed: the week's report for a person, or the
    /// month's eight numbers for a machine.
    #[arg(long, default_value = "md", value_parser = ["md", "json"])]
    pub format: String,

    /// Also run every documented example and fold the red ones in.
    /// Without it they are not measured, and the queue says so rather
    /// than reporting none: the runner builds a sandbox per fixture and
    /// costs minutes, which is not what a weekly reading should cost.
    #[arg(long)]
    pub examples: bool,

    /// The bar the coverage and style readings state. It changes what
    /// the report says the target is, never whether this command
    /// succeeds.
    #[arg(long, default_value_t = vibe_doc::coverage::FULL_COVERAGE, value_name = "PERCENT")]
    pub min: u8,

    /// The debt file to count `docs:` lines in. Defaults to `BACKLOG.md`
    /// at the root of the checkout this runs in.
    #[arg(long, value_name = "PATH")]
    pub backlog: Option<PathBuf>,

    /// The journal to count entries owing a decision in. Defaults to
    /// `JOURNAL.md` in the documentation package.
    #[arg(long, value_name = "PATH")]
    pub journal: Option<PathBuf>,

    /// The `vibe` binary the examples run and the surface is read from.
    /// Defaults to the running one.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,

    /// Where example sandboxes are built.
    #[arg(long, value_name = "PATH")]
    pub sandbox: Option<PathBuf>,

    /// Seconds one documented command may take before it is killed.
    #[arg(long, default_value_t = 300, value_name = "SECONDS")]
    pub timeout: u64,
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

    /// Hand every documented `prompt` to the configured agent in a clean
    /// sandbox, then run the page's own asserts on what it left. An
    /// agent says something different every time, so the page states
    /// what must be TRUE afterwards and this checks that. Not part of
    /// the panel: it calls a real agent and takes real time.
    #[arg(long)]
    pub prompts: bool,

    /// With `--prompts`, the command the prompt text is handed to,
    /// overriding the package's own `[doc.prompts] runner`. One corpus
    /// through two agents is how a prompt that only works on one of them
    /// is caught.
    #[arg(long, value_name = "COMMAND")]
    pub runner: Option<String>,

    /// With `--prompts`, take this many prompts instead of all of them.
    /// The choice is random in the sense that it is not the first N of
    /// the corpus, and fixed in the sense that two runs a month apart
    /// take the same prompts.
    #[arg(long, value_name = "N")]
    pub sample: Option<usize>,

    /// Lint the prose against the style law: the tics on the package's
    /// list for its language, sentence and paragraph length by block
    /// kind, a glossary term used before anybody introduced it, three
    /// terms in one sentence, a heading the law forbids by name. It
    /// reports and never edits.
    #[arg(long)]
    pub style: bool,

    /// With `--coverage` and `--style`, the percentage that counts as a
    /// pass — of audience pairs told, and of pages that carry no style
    /// error. The standing bar is everything told and every page clean; a
    /// lower one is for the intermediate runs of a campaign that is still
    /// writing the pages.
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
