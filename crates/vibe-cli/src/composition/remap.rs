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
//! The two package layers of `PROP-066#layers` are ranked by the layer
//! enum and produced by nothing: remap packages, their offers and the
//! consent that activates them are R-2.
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
            target_ref: None,
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
pub(crate) fn install(flags: &[PathBuf], env_list: Option<String>, release_origin: Option<String>) {
    let env = RemapEnv {
        flags: flags.to_vec(),
        env: env_list,
        user_settings: vibe_core::settings::settings_toml_path(),
        project_root: std::env::current_dir().ok(),
        global_registry: vibe_core::settings::registry_config_path(),
        release_origin,
    };
    match build(&env) {
        Ok((locator, warnings)) => {
            for warning in warnings {
                eprintln!("vibe: warning: remap layer skipped: {warning}");
            }
            let _ = vibe_core::remap::install(locator);
        }
        Err(error) => {
            eprintln!("vibe: {error:#}");
            // A named remap file that cannot be read leaves the Locator
            // uninstalled, so `active()` is the empty one and the run
            // would fetch from the declared origins — exactly what the
            // file existed to prevent. The process stops instead.
            std::process::exit(crate::exit_code::USAGE as i32);
        }
    }
}
