//! Building the invocation's remap Locator (`PROP-066#layers`).
//!
//! The consent-free layers, lowest first:
//!
//! 1. The built-in single-file endpoints of PROP-002's HTTPS fast path,
//!    as default `raw-file` rows — the weakest thing in the room, so any
//!    row a user writes replaces them.
//! 2. **Local**: PROP-002's `[[mirror]]` / `[[override]]` tables from the
//!    machine `registry.toml` and the project `vibe.toml`, then the
//!    `[[remap]]` tables of `~/.vibe/settings.toml` (PROP-040 L1) and
//!    `.vibe/settings.local.toml` (L3). Nothing a user already wrote
//!    stops working, and nothing here needs anyone's consent — the user
//!    wrote it.
//! 3. **Invocation**: `$VIBEVM_RELEASE_ORIGIN` (the one the install
//!    scripts already honour, read as a row so the `vibe self` they
//!    bootstrap agrees with them), then `--remap <file>` (repeatable) and
//!    `VIBE_REMAP=<file>[,<file>]`.
//!
//! Beneath the local layer sit the two package layers of
//! `PROP-066#layers`: the remap packages this user accepted in their home,
//! and the ones the project requires and the lock pins. They are the
//! weakest rows that are not built in, so anything a developer wrote on
//! this machine still wins — and they are produced only after the
//! acceptance ledger says yes, which is what makes an offer an offer.
//!
//! A malformed or missing *settings* file is a warning and an absent
//! layer, the way every optional configuration layer behaves. A malformed
//! or missing `--remap` / `VIBE_REMAP` file is a hard error: the operator
//! named it on this invocation, so swallowing a typo would silently fetch
//! from the public hosts the file existed to replace.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#layers");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_core::manifest::{Manifest, MirrorSection, OverrideSection, RegistrySection};
use vibe_core::remap::{
    Layer, LoadedSource, Locator, default_raw_file_rows, env_remap_files, rows_from_file,
    rows_from_mirrors, rows_from_overrides, rows_from_settings,
};

/// The release origin `install.sh` / `install.ps1` declare and the
/// `vibe self` URLs of `commands::vvm::bundle` are built from. Named here
/// because this is where `$VIBEVM_RELEASE_ORIGIN` becomes a row; keep the
/// three spellings in step.
pub(crate) const CANONICAL_RELEASE_ORIGIN: &str = "https://github.com/vibevm/vibevm/releases";

/// The environment variable the install scripts already honour, read here
/// so a bootstrap started by one of them inherits the same redirection its
/// own downloads used (`PROP-066#locator`).
pub(crate) const RELEASE_ORIGIN_ENV: &str = "VIBEVM_RELEASE_ORIGIN";

/// What the Locator is built from. The two environment values arrive as
/// parameters, so the CLI's one sanctioned ambient-env root keeps every
/// `env::var`; the rest are paths this module resolves.
pub(crate) struct RemapEnv {
    /// The remap packages whose rows this run carries, lowest layer
    /// first: the user home's accepted offers, then the ones the project
    /// requires (`PROP-066#layers`). Empty until consent says otherwise,
    /// which is what makes an offer an offer.
    pub accepted: Vec<(Layer, vibe_core::remap::RemapPackage)>,
    /// `--remap <file>`, in the order given.
    pub flags: Vec<PathBuf>,
    /// The raw `VIBE_REMAP` value, if the variable is set and non-empty.
    pub env: Option<String>,
    /// `~/.vibe/settings.toml` (PROP-040 L1), when the home resolves.
    pub user_settings: Option<PathBuf>,
    /// The project root `.vibe/settings.local.toml` lives under.
    pub project_root: Option<PathBuf>,
    /// The machine `~/.vibe/registry.toml`, when the home resolves.
    pub global_registry: Option<PathBuf>,
    /// `$VIBEVM_RELEASE_ORIGIN`, when the operator set it.
    pub release_origin: Option<String>,
}

/// Build the invocation's Locator from `env` and the project manifest.
///
/// Warnings about optional layers are returned rather than printed, so the
/// caller decides where they go and `--json` keeps one envelope.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
pub(crate) fn build(env: &RemapEnv) -> Result<(Locator, Vec<String>)> {
    let mut warnings = Vec::new();
    let mut sources = vec![LoadedSource {
        layer: Layer::UserPackage,
        source: "built-in".into(),
        rows: default_raw_file_rows(&vibe_registry::git_backend::shell::raw_http::default_bases()),
        policy: Default::default(),
    }];

    // The two package layers, beneath everything a user wrote themselves:
    // a package's rows are the weakest thing that is not built in, so a
    // local row or a `--remap` file always wins over an offer
    // (`PROP-066#layers`).
    for (layer, package) in &env.accepted {
        match package.loaded(*layer) {
            Ok(loaded) => sources.push(loaded),
            // A malformed row inside an accepted package is a warning and
            // an absent layer, not a dead command: `vibe check` is where
            // a package's rows are judged, and refusing every install
            // over one bad row in an accepted offer would be worse than
            // the row.
            Err(error) => warnings.push(format!("`{}`: {error}", package.label())),
        }
    }

    let (registries, mirrors, overrides) = registry_tables(env, &mut warnings);
    sources.push(LoadedSource {
        layer: Layer::Local,
        source: "[[mirror]] / [[override]]".into(),
        rows: mirror_and_override_rows(&registries, &mirrors, &overrides),
        policy: Default::default(),
    });

    for (path, label) in settings_files(env) {
        match optional_settings(&path, &label) {
            Ok(Some(loaded)) => sources.push(loaded),
            Ok(None) => {}
            Err(warning) => warnings.push(warning),
        }
    }

    // `$VIBEVM_RELEASE_ORIGIN` is an invocation-layer row, which is what
    // it has always been in substance: `install.sh` and `install.ps1`
    // declare it and build every download from it, and the `vibe self`
    // they bootstrap has to agree with them or the first `self update`
    // would go back to the public host the operator just replaced. As a
    // row it is also visible in `vibe remap list` and explainable, which
    // an environment variable alone never was.
    if let Some(origin) = env
        .release_origin
        .as_deref()
        .map(str::trim)
        .filter(|origin| !origin.is_empty() && *origin != CANONICAL_RELEASE_ORIGIN)
    {
        let row = vibe_core::remap::RemapRow {
            key: vibe_core::remap::RemapKey::Prefix(CANONICAL_RELEASE_ORIGIN.to_owned()),
            target: vibe_core::remap::RemapTarget::Url(origin.to_owned()),
            scope: None,
            reason: Some(format!("${RELEASE_ORIGIN_ENV}")),
            from: vibe_core::remap::Provenance::new(Layer::Invocation, RELEASE_ORIGIN_ENV, 0),
            from_package: None,
            target_ref: None,
            only: None,
        };
        sources.push(LoadedSource {
            layer: Layer::Invocation,
            source: RELEASE_ORIGIN_ENV.into(),
            rows: vec![row],
            policy: Default::default(),
        });
    }

    // The invocation layer: flags first, then the environment list, so a
    // flag typed on this command line is the later — and therefore the
    // winning — row of the layer.
    for path in env
        .flags
        .iter()
        .cloned()
        .chain(env.env.as_deref().map(env_remap_files).unwrap_or_default())
    {
        sources.push(
            rows_from_file(&path, Layer::Invocation)
                .with_context(|| format!("reading the remap file `{}`", path.display()))?,
        );
    }

    let declared = registries
        .iter()
        .map(|registry| (registry.name.clone(), registry.url.clone()))
        .collect();
    Ok((Locator::from_sources(sources, declared), warnings))
}

/// The L1 and L3 settings files, with the label each row's provenance
/// carries. L2 (`<repo>/.vibe/settings.toml`, committed) is deliberately
/// absent: a repo-shared file is not this user's own machine, and
/// `PROP-066#layers` gives a project's offer to the project-package layer,
/// behind consent — which is R-2.
fn settings_files(env: &RemapEnv) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    if let Some(path) = &env.user_settings {
        files.push((path.clone(), path.display().to_string()));
    }
    if let Some(root) = &env.project_root {
        let path = root.join(".vibe").join("settings.local.toml");
        files.push((path.clone(), path.display().to_string()));
    }
    files
}

/// Read one optional settings file's `[[remap]]` rows. `Ok(None)` when the
/// file is absent or carries none; `Err(warning)` when it cannot be read
/// or parsed, which is a warning for the caller, never a failed command.
fn optional_settings(path: &Path, label: &str) -> Result<Option<LoadedSource>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("could not read `{label}`: {error}"))?;
    let table: toml::Table =
        toml::from_str(&body).map_err(|error| format!("`{label}` is malformed: {error}"))?;
    let loaded = rows_from_settings(&table, Layer::Local, label)
        .map_err(|error| format!("`{label}`: {error}"))?;
    if loaded.rows.is_empty() && loaded.policy == Default::default() {
        return Ok(None);
    }
    Ok(Some(loaded))
}

/// PROP-002's tables as rows: the mirrors first, then the overrides, so an
/// `[[override]]`'s `package` key is the later row of the pair as well as
/// the more specific one.
fn mirror_and_override_rows(
    registries: &[RegistrySection],
    mirrors: &[MirrorSection],
    overrides: &[OverrideSection],
) -> Vec<vibe_core::remap::RemapRow> {
    let mut rows = rows_from_mirrors(mirrors, registries, Layer::Local, "[[mirror]]");
    let offset = rows.len();
    for (position, mut row) in rows_from_overrides(overrides, Layer::Local, "[[override]]")
        .into_iter()
        .enumerate()
    {
        // One source, one index sequence: the layer's tie-break is "the
        // later row", and two sources sharing a `LoadedSource` must not
        // both start at zero.
        row.from.index = offset + position;
        rows.push(row);
    }
    rows
}

/// The effective registry tables: the project manifest's merged with the
/// machine-global file's, exactly as resolution merges them (PROP-002
/// §2.2.2). Both layers are optional; a failure to read either is a
/// warning and an absent table.
fn registry_tables(
    env: &RemapEnv,
    warnings: &mut Vec<String>,
) -> (
    Vec<RegistrySection>,
    Vec<MirrorSection>,
    Vec<OverrideSection>,
) {
    let global = match env.global_registry.as_deref() {
        Some(path) => match vibe_core::GlobalRegistryConfig::load_from(path) {
            Ok(config) => config,
            Err(error) => {
                warnings.push(error.to_string());
                Default::default()
            }
        },
        None => Default::default(),
    };
    let project = env
        .project_root
        .as_deref()
        .map(|root| root.join(Manifest::FILENAME))
        .filter(|path| path.exists())
        .and_then(|path| {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|body| Manifest::parse_str(&body).map_err(|e| e.to_string()))
            {
                Ok(manifest) => Some(manifest),
                Err(error) => {
                    warnings.push(format!("`{}` is malformed: {error}", path.display()));
                    None
                }
            }
        });
    match project {
        Some(manifest) => {
            let effective = vibe_core::merge_effective(&manifest, &global);
            (effective.registries, effective.mirrors, effective.overrides)
        }
        None => (global.registries, global.mirrors, global.overrides),
    }
}

/// Build the invocation's remap Locator and install it
/// (`PROP-066#locator`), so every fetch path resolves its artifacts
/// through one point.
///
/// `main` reads the two environment variables and passes them in, so the
/// one sanctioned ambient-env root keeps every `env::var` of the CLI; the
/// settings home and the current directory are resolved here because
/// neither is an environment read. A layer that cannot be read is a
/// warning on stderr
/// and an absent layer — the behaviour of every optional configuration
/// layer; a `--remap` file that cannot be read is fatal, because the
/// operator named it on this command line and a run that quietly fetched
/// from the public hosts instead would be the worst possible answer.
pub(crate) fn install(ctx: &crate::output::Context, inputs: Inputs<'_>) -> Result<()> {
    let project_root = std::env::current_dir().ok();
    let accepted = accepted_offers(ctx, &inputs, project_root.as_deref())?;
    let env = RemapEnv {
        accepted,
        flags: inputs.flags.to_vec(),
        env: inputs.env_list,
        user_settings: vibe_core::settings::settings_toml_path(),
        project_root,
        global_registry: vibe_core::settings::registry_config_path(),
        release_origin: inputs.release_origin,
    };
    let (locator, warnings) = build(&env)?;
    for warning in warnings {
        eprintln!(
            "{}",
            crate::output::language::warning(&format!("remap layer skipped: {warning}"))
        );
    }
    let _ = vibe_core::remap::install(locator);
    Ok(())
}

/// What the composition root hands this module: the invocation's flags,
/// the two environment values, and whether this verb resolves artifacts at
/// all.
pub(crate) struct Inputs<'a> {
    /// `--remap <file>`, in the order given.
    pub flags: &'a [PathBuf],
    /// The raw `VIBE_REMAP` value.
    pub env_list: Option<String>,
    /// `$VIBEVM_RELEASE_ORIGIN`.
    pub release_origin: Option<String>,
    /// How this run answers an unanswered offer.
    pub consent: crate::commands::remap::ConsentFlags,
    /// The verb this invocation runs. Whether an unanswered offer is a
    /// question at all depends on it (`PROP-066#consent`), and the
    /// decision is taken here rather than at the composition root
    /// because the cheap half of it — "is there a pending offer?" —
    /// has to come first.
    pub command: &'a crate::cli::Command,
    /// The run's network posture, for the diagnostics the question shows.
    pub offline: bool,
}

/// The accepted offers of this run, per layer, asking or refusing as
/// `PROP-066#consent` requires.
///
/// A verb that resolves no artifact reads the ledger and asks nothing: the
/// rows of an already accepted package still apply (nothing about this run
/// changes what the user agreed to), and an unanswered one stays
/// unanswered.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers-packages")]
fn accepted_offers(
    ctx: &crate::output::Context,
    inputs: &Inputs<'_>,
    project_root: Option<&Path>,
) -> Result<Vec<(Layer, vibe_core::remap::RemapPackage)>> {
    let mut home = match crate::commands::remap::RemapHome::open() {
        Ok(home) => home,
        // No resolvable home is no accepted package, which is the
        // pre-PROP-066 behaviour exactly; a command that needs the home
        // (`remap accept`) reports the missing home itself.
        Err(_) => return Ok(Vec::new()),
    };
    let accepted = if asks_about_offers(&home, inputs) {
        crate::commands::remap::resolve_consent(ctx, &mut home, &inputs.consent, inputs.offline)?
    } else {
        // A verb that applies no offer asks nothing and refuses nothing:
        // the rows of an already accepted package still apply (this run
        // changes nothing about what the user agreed to), and an
        // unanswered one stays unanswered for the run that would use it.
        home.accepted()
    };
    // A package the project requires is the higher of the two package
    // layers (`PROP-066#layers`), and the lock is what says which ones
    // those are — pinned there so every clone of the project is offered
    // the same list.
    let required = project_required(project_root);
    Ok(accepted
        .into_iter()
        .map(|package| {
            let layer = if required.iter().any(|key| key == &package.key) {
                Layer::ProjectPackage
            } else {
                Layer::UserPackage
            };
            (layer, package)
        })
        .collect())
}

/// The remap packages the project's lock pins (`PROP-066#layers-packages`).
fn project_required(project_root: Option<&Path>) -> Vec<String> {
    let Some(root) = project_root else {
        return Vec::new();
    };
    let lock = root.join(vibe_core::manifest::Lockfile::FILENAME);
    if !lock.exists() {
        return Vec::new();
    }
    vibe_core::manifest::Lockfile::read(&lock)
        .map(|lockfile| lockfile.remaps.into_iter().map(|row| row.package).collect())
        .unwrap_or_default()
}

/// Whether an unanswered remap offer is a question for this run
/// (`PROP-066#consent`).
///
/// Three tests, cheapest first, because the third costs a resolution: the
/// verb has to resolve artifacts at all; there has to be an offer nobody
/// has answered; and the verb must not be one that only records or
/// retires an offer. The order is what keeps the probe off every ordinary
/// run — a home with nothing pending never reaches it.
fn asks_about_offers(home: &crate::commands::remap::RemapHome, inputs: &Inputs<'_>) -> bool {
    resolves_artifacts(inputs.command)
        && !home.pending().is_empty()
        && !only_records_remap_offers(inputs.command, inputs.offline)
}

/// Whether this verb resolves artifacts, and therefore whether an
/// unanswered remap offer is a question for it (PROP-066 `#consent`).
///
/// The list is the one `#consent` names — `install`, `update`,
/// `uninstall`, `app plan`, `cache add`, `self update` — plus every other
/// verb that reaches a fetch path: the lifecycle phases, `reinstall`,
/// `outdated`, `search`, `doc`, `skill`, `mcp`. The verbs left out are the
/// ones that must never ask: `remap` itself (the operator is in the middle
/// of answering), and everything that reads only local state.
fn resolves_artifacts(cmd: &crate::cli::Command) -> bool {
    use crate::cli::AppCommand;
    use crate::cli::CacheSubcommand;
    use crate::cli::Command;
    // `vibe app launchers --json` and `vibe cache check` read local state
    // only; a desktop shell polls them, and a pending offer must not turn
    // every poll into a question (or, unattended, into `consent_required`).
    if let Command::App(args) = cmd {
        return matches!(
            args.command,
            AppCommand::Plan { .. }
                | AppCommand::Modify { .. }
                | AppCommand::Outdated { .. }
                | AppCommand::Rollback { .. }
        );
    }
    if let Command::Cache(args) = cmd {
        return matches!(args.command, CacheSubcommand::Add(_));
    }
    matches!(
        cmd,
        Command::Install(_)
            | Command::Update(_)
            | Command::Uninstall(_)
            | Command::Reinstall(_)
            | Command::Outdated(_)
            | Command::Search(_)
            | Command::Doc(_)
            | Command::Skill(_)
            | Command::Mcp(_)
            | Command::Vvm(_)
            | Command::Validate(_)
            | Command::Generate(_)
            | Command::Build(_)
            | Command::Test(_)
            | Command::Create(_)
            | Command::Verify(_)
            | Command::Package(_)
            | Command::Deploy(_)
    )
}

/// Whether this run only RECORDS remap offers rather than applying any
/// (PROP-066 `#consent`).
///
/// Installing a remap package, or removing one, applies no row: the first
/// writes an offer nobody has answered yet and the second asks its own
/// question (`#uninstall`). Neither may be blocked by a *different*
/// unanswered offer, or a home with one pending offer could never install
/// a second remap package without first answering the first.
///
/// The kind is read from the manifest the registry would serve, through
/// the consent-free layers — an unaccepted row must not be used to fetch
/// anything, so resolving the probe without it is not a loss. A probe that
/// cannot answer says `false`, which only means the run asks its question
/// as any other install does.
fn only_records_remap_offers(cmd: &crate::cli::Command, offline: bool) -> bool {
    match cmd {
        crate::cli::Command::Install(args) if args.global => {
            crate::commands::remap::claims_install(args, offline).is_some()
        }
        crate::cli::Command::Uninstall(args) => {
            crate::commands::remap::claims_uninstall(&args.package)
        }
        _ => false,
    }
}
