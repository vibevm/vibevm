//! `vibe self` — the VibeVM Version Manager (VVM): build, install, switch,
//! and remove vibevm's own versions on this machine (PROP-019). A
//! standalone-mode capability — pure algorithm, no LLM (PROP-019 §2.1).
//!
//! Dispatches every `vibe self` verb over the instance layout and the live
//! `current` pointer (PROP-019 §2.4, §2.5).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

mod builder;
mod bundle;
mod doctor;
mod embedded;
mod env;
mod error;
mod git;
mod import;
mod install;
mod model;
mod placer;
mod provenance;
mod relocate;
mod remove;
pub(crate) mod selfloc;
mod source;
mod store;
mod tools;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::Result;
use dialoguer::Confirm;

use crate::cli::{
    ForcedKind, VvmArgs, VvmEnvArgs, VvmImportArgs, VvmInstallArgs, VvmSubcommand, VvmUseArgs,
};
use crate::output;

use error::VvmError;
use model::{InstallRecord, State, VersionId};
use provenance::running_record;
use store::VersionStore;

pub(crate) use embedded::embedded_root_at;
pub use selfloc::{derive_self, same_location};

/// Env var naming the install base (defaults to `~/.vibe`); the
/// VVM root is `$VIBEVM_INSTALL_ROOT/opt`. Read at the composition root and
/// overridden in tests to isolate installs under a temp dir (PROP-019 §2.4).
pub const VIBEVM_INSTALL_ROOT_ENV: &str = "VIBEVM_INSTALL_ROOT";
/// Env var advertising the active version's prefix — advisory only; the
/// truth is the `current` file + `current_exe` (PROP-019 §2.5). Read for the
/// divergence warning and `vibe vars`.
pub const VIBEVM_HOME_ENV: &str = "VIBEVM_HOME";

/// Ambient environment VVM needs, resolved at the composition root
/// (`main.rs`) and threaded in — the domain never reads the process env
/// itself (PROP-019 §2.1). The *active* version is the `current` file, not
/// an env var (PROP-019 §2.5).
#[derive(Debug, Clone, Default)]
pub struct VvmEnv {
    /// The resolved VVM root — `$VIBEVM_INSTALL_ROOT/opt`, defaulting to
    /// `~/.vibe/opt`.
    pub root: Option<PathBuf>,
    /// The current working directory — for in-tree source detection on
    /// `self install` (PROP-019 §2.7).
    pub cwd: Option<PathBuf>,
    /// The user's real home directory — for locating the shell rc to edit on
    /// POSIX activation (PROP-019 §2.6).
    pub home: Option<PathBuf>,
    /// `$SHELL` — for shell detection (PROP-019 §2.6).
    pub shell: Option<String>,
    /// `$PATH` — to check whether the shim dir is reachable (`doctor`).
    pub path_var: Option<String>,
}

impl VvmEnv {
    fn store(&self) -> Result<VersionStore, VvmError> {
        let root = self.root.clone().ok_or(VvmError::NoRoot)?;
        Ok(VersionStore::new(root))
    }
}

/// Resolve the VVM root without reading ambient process state.
///
/// A running managed binary owns its already-derived root. Otherwise an
/// explicit `$VIBEVM_INSTALL_ROOT` names the install base, and the default
/// base is `~/.vibe`.
pub(crate) fn resolve_root(
    managed_root: Option<PathBuf>,
    install_base_override: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    managed_root.or_else(|| {
        install_base_override
            .or_else(|| home.map(|path| path.join(".vibe")))
            .and_then(|base| absolute_lexical(&base, &std::env::current_dir().ok()?))
            .map(|base| base.join("opt"))
    })
}

fn absolute_lexical(path: &Path, cwd: &Path) -> Option<PathBuf> {
    let combined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in combined.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized.is_absolute().then_some(normalized)
}

pub fn run(ctx: &output::Context, args: VvmArgs, env: VvmEnv) -> Result<()> {
    match args.command {
        VvmSubcommand::Install(a) => run_install_cmd(ctx, &env, a),
        VvmSubcommand::Import(a) => run_import_cmd(ctx, &env, a),
        VvmSubcommand::Bootstrap(a) => bundle::run_bootstrap_cmd(ctx, &env, a),
        VvmSubcommand::Update(a) => bundle::run_update_cmd(ctx, &env, a),
        VvmSubcommand::Reinstall(a) => bundle::run_reinstall_cmd(ctx, &env, a),
        VvmSubcommand::Use(a) => run_use_cmd(ctx, &env, a),
        VvmSubcommand::Rollback => run_rollback_cmd(ctx, &env),
        VvmSubcommand::Ls => provenance::run_ls_cmd(ctx, &env),
        VvmSubcommand::Current => provenance::run_current_cmd(ctx, &env),
        VvmSubcommand::Which(a) => provenance::run_which_cmd(ctx, &env, a),
        VvmSubcommand::Source => provenance::run_source_cmd(ctx, &env),
        VvmSubcommand::Doctor(a) => doctor::run_doctor_cmd(ctx, &env, a),
        VvmSubcommand::Remove(a) => remove::run_remove_cmd(ctx, &env, a),
        VvmSubcommand::Gc(a) => remove::run_gc_cmd(ctx, &env, a),
        VvmSubcommand::Env(a) => run_env_cmd(&env, a),
        VvmSubcommand::Relocate(a) => {
            let store = env.store()?;
            let _lock = install::InstallLock::acquire(&store)?;
            relocate::run_relocate_cmd(ctx, &env, a)
        }
    }
}

fn run_import_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmImportArgs) -> Result<()> {
    let store = env.store()?;
    let _lock = install::InstallLock::acquire(&store)?;
    let profile = model::Profile::parse(&args.profile)?;
    let now = chrono::Utc::now().to_rfc3339();
    let req = import::ImportRequest {
        executable: &args.path,
        tag: &args.tag,
        commit: args.commit.as_deref(),
        profile,
        replace_candidate: args.replace_candidate,
        now: &now,
    };
    let outcome = import::perform_import(ctx, &store, &req)?;
    debug_assert_eq!(
        outcome.home,
        store.instance_dir(&outcome.record.version_id(), outcome.record.instance)
    );
    let _ = outcome.reused;
    if args.activate {
        ensure_activatable(&store, &outcome.record)?;
        activate_record(ctx, env, &store, &outcome.record, "self:import")?;
    }
    Ok(())
}

fn run_install_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmInstallArgs) -> Result<()> {
    let store = env.store()?;
    let profile = resolve_profile(&args)?;
    let selector = model::Selector::parse(&args.selector, forced_kind(&args.kind))?;
    if matches!(&selector, model::Selector::Exact(_)) {
        return Err(VvmError::ExactInstanceInstall.into());
    }
    let now = chrono::Utc::now().to_rfc3339();

    // Source comes from running provenance, a bare dev cwd, or the managed mirror.
    let running = running_record(&store)?;
    let binary_execution = args
        .mirror
        .is_none()
        .then(|| {
            running
                .as_ref()
                .filter(|record| record.origin == model::Origin::Binary)
        })
        .flatten();
    if let Some(record) = binary_execution {
        match binary_lane(&selector) {
            Some(BinaryLane::Version(version)) => {
                return bundle::install_binary_version(ctx, env, &store, &version, args.force);
            }
            Some(BinaryLane::Newest) => {
                return bundle::install_newest_release(
                    ctx,
                    env,
                    &store,
                    record,
                    args.force,
                    "self:install",
                );
            }
            Some(BinaryLane::NoRelease) => return Err(VvmError::BinaryFetchUnavailable.into()),
            None => {}
        }
    }
    let _lock = install::InstallLock::acquire(&store)?;
    let running_tree = running
        .as_ref()
        .filter(|record| record.origin == model::Origin::External)
        .and_then(|record| record.source_path.as_deref())
        .and_then(|path| source::find_source_root(Path::new(path)));
    let in_tree = running_tree
        .or_else(provenance::executable_source_root)
        .or_else(|| {
            running
                .is_none()
                .then(|| env.cwd.as_deref().and_then(source::find_source_root))
                .flatten()
        });
    let prefer_in_tree = matches!(selector, model::Selector::Latest) && args.mirror.is_none();

    let (source_dir, resolved, origin, source_path) =
        if let (Some(root), true) = (in_tree.as_ref(), prefer_in_tree) {
            let resolved = source::label_in_tree(root)?;
            (
                root.clone(),
                resolved,
                model::Origin::External,
                Some(source::external_path(root)),
            )
        } else if args.mirror.is_none()
            && let Some(root) = source::linked_source(&store, &selector, &args.selector)?
        {
            ctx.step(&format!("rebuilding from linked source {}", root.display()));
            let resolved = source::label_in_tree(&root)?;
            let path = source::external_path(&root);
            (root, resolved, model::Origin::External, Some(path))
        } else {
            let mirror = source::choose_mirror(ctx, args.mirror.as_deref())?;
            ctx.step(&format!("updating managed clone from {}", mirror.url()));
            let outcome = source::prepare_from_mirror(&store, mirror.url(), &selector)?;
            (
                outcome.src_dir,
                outcome.resolved,
                model::Origin::Managed,
                None,
            )
        };

    let req = install::InstallRequest {
        resolved: &resolved,
        profile,
        force: args.force,
        now: &now,
        origin,
        source_path,
    };
    let outcome = install::perform_install(ctx, &store, &source_dir, &req, &builder::CargoBuilder)?;
    debug_assert_eq!(
        outcome.home,
        store.instance_dir(&outcome.record.version_id(), outcome.record.instance)
    );
    let _ = outcome.reused;
    activate_record(ctx, env, &store, &outcome.record, "self:install")
}

/// The release lane a selector takes on a managed binary execution, decided
/// before any source build is considered (PROP-019 §2.2).
#[derive(Debug, PartialEq, Eq)]
enum BinaryLane {
    /// One named release, installed from its own release directory.
    Version(String),
    /// Whatever release is newest right now.
    Newest,
    /// A source-branch selector that no release publishes.
    NoRelease,
}

/// Classify a selector for a binary execution; `None` falls through to the
/// source lane, exactly as an explicit `--mirror` does.
///
/// `stable` IS the newest release (PROP-019 `##SEL-STABLE`), which is what
/// `self update` resolves on this same execution — so both enter one
/// function, rather than `stable` quietly turning into a source build for a
/// selector the release lane can satisfy exactly.
fn binary_lane(selector: &model::Selector) -> Option<BinaryLane> {
    match selector {
        model::Selector::Explicit(id) if id.kind == model::Kind::Tag => Some(BinaryLane::Version(
            id.id.strip_prefix('v').unwrap_or(&id.id).to_string(),
        )),
        model::Selector::Stable => Some(BinaryLane::Newest),
        model::Selector::Latest => Some(BinaryLane::NoRelease),
        _ => None,
    }
}

fn resolve_profile(args: &VvmInstallArgs) -> Result<model::Profile, model::ModelError> {
    if args.release {
        return Ok(model::Profile::Release);
    }
    match &args.profile {
        Some(p) => model::Profile::parse(p),
        None => Ok(model::DEFAULT_PROFILE),
    }
}

fn forced_kind(k: &ForcedKind) -> Option<model::Kind> {
    if k.tag {
        Some(model::Kind::Tag)
    } else if k.branch {
        Some(model::Kind::Branch)
    } else if k.commit {
        Some(model::Kind::Commit)
    } else {
        None
    }
}

fn run_use_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmUseArgs) -> Result<()> {
    let store = env.store()?;
    let selector = model::Selector::parse(&args.selector, forced_kind(&args.kind))?;

    if args.eval {
        let state = store.load_state()?;
        let rec = resolve_installed(&state, &selector, &args.selector)?;
        ensure_activatable(&store, &rec)?;
        let home = store.instance_dir(&rec.version_id(), rec.instance);
        let shell = env::Shell::detect(env.shell.as_deref());
        // Print only the line to eval in the current shell; persist nothing.
        println!("{}", shell.export_line(&home));
        return Ok(());
    }

    let _lock = install::InstallLock::acquire(&store)?;
    let state = store.load_state()?;
    let rec = resolve_installed(&state, &selector, &args.selector)?;
    ensure_activatable(&store, &rec)?;
    activate_record(ctx, env, &store, &rec, "self:use")
}

fn run_rollback_cmd(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let _lock = install::InstallLock::acquire(&store)?;
    let record = store.previous()?.ok_or(VvmError::NoRollback)?;
    ensure_activatable(&store, &record).map_err(|_| VvmError::NoRollback)?;
    activate_record(ctx, env, &store, &record, "self:rollback")
}

fn ensure_activatable(store: &VersionStore, record: &InstallRecord) -> Result<(), VvmError> {
    let intact = if record.origin == model::Origin::Binary && record.source_path.is_some() {
        bundle::installed_bundle_intact(store, record)
    } else {
        placer::installed_files_match(store, record)
    };
    if intact {
        Ok(())
    } else {
        Err(VvmError::CorruptInstance {
            selector: record.selector().to_string(),
        })
    }
}

fn activate_record(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    record: &InstallRecord,
    command: &str,
) -> Result<()> {
    let id = record.version_id();
    let home = store.instance_dir(&id, record.instance);
    let shell = env::Shell::detect(env.shell.as_deref());
    let persister = make_persister(env, shell)?;
    let path_ready = path_has_dir(env.path_var.as_deref(), &store.shim_dir());
    let activated = env::activate_instance(store, &home, persister.as_ref())?;

    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": command,
            "active": id.to_string(),
            "selector": record.selector().to_string(),
            "instance": record.instance,
            "home": home.display().to_string(),
            "path_on_current_process": path_ready,
            "durable_path_changed": activated.path == env::Persisted::Changed,
            "advisory_home_warning": activated.advisory_home_warning,
        }));
    }
    ctx.summary(&format!("active → {}", record.selector()));
    ctx.summary("  switched live; the next `vibe` in this shell uses it");
    if path_ready {
        ctx.summary("  PATH is ready in this process");
    } else {
        ctx.summary(&format!(
            "  external tools: {}",
            persister.activation_hint()
        ));
    }
    if let Some(warning) = activated.advisory_home_warning {
        ctx.summary(&format!(
            "  warning: live pointer switched; advisory VIBEVM_HOME update failed: {warning}"
        ));
    }
    Ok(())
}

fn run_env_cmd(env: &VvmEnv, args: VvmEnvArgs) -> Result<()> {
    let shell = match args.shell.as_deref() {
        Some(s) => env::Shell::parse(s)?,
        None => env::Shell::detect(env.shell.as_deref()),
    };
    let store = env.store()?;
    let home = match args.selector.as_deref() {
        Some(raw) => {
            let state = store.load_state()?;
            let selector = model::Selector::parse(raw, forced_kind(&args.kind))?;
            let rec = resolve_installed(&state, &selector, raw)?;
            ensure_activatable(&store, &rec)?;
            store.instance_dir(&rec.version_id(), rec.instance)
        }
        None => {
            let rec = store.active()?.ok_or(VvmError::NoActiveVersion)?;
            ensure_activatable(&store, &rec)?;
            store.instance_dir(&rec.version_id(), rec.instance)
        }
    };
    println!("{}", shell.export_line(&home));
    Ok(())
}

/// Map a selector onto the newest *installed* instance of its id (PROP-019
/// §2.3, §2.11).
fn resolve_installed(
    state: &State,
    selector: &model::Selector,
    raw: &str,
) -> Result<InstallRecord, VvmError> {
    use model::{Kind, Selector, VersionId};
    match selector {
        Selector::Latest => {
            latest_of(state, &VersionId::new(Kind::Branch, "main")).ok_or_else(|| {
                VvmError::NotInstalled {
                    detail: "`latest` is not installed".to_string(),
                }
            })
        }
        Selector::Explicit(id) => latest_of(state, id).ok_or_else(|| VvmError::NotInstalled {
            detail: format!("`{id}` is not installed (try `vibe self install {raw}`)"),
        }),
        Selector::Exact(exact) => state
            .installs
            .iter()
            .find(|record| {
                record.version_id() == exact.version && record.instance == exact.instance
            })
            .cloned()
            .ok_or_else(|| VvmError::NotInstalled {
                detail: format!("exact local instance `{exact}` is not installed"),
            }),
        Selector::Stable => highest_tag_record(state).ok_or_else(|| VvmError::NotInstalled {
            detail: "no installed release tag satisfies `stable`".to_string(),
        }),
        Selector::Ambiguous(name) => {
            by_precedence_record(state, name).ok_or_else(|| VvmError::NotInstalled {
                detail: format!("no installed version named `{name}`"),
            })
        }
    }
}

/// The newest instance of a version id.
fn latest_of(state: &State, id: &VersionId) -> Option<InstallRecord> {
    state
        .installs
        .iter()
        .filter(|r| &r.version_id() == id)
        .max_by_key(|r| r.instance)
        .cloned()
}

/// The newest instance of the highest installed semver tag (PROP-019 §2.3).
fn highest_tag_record(state: &State) -> Option<InstallRecord> {
    state
        .installs
        .iter()
        .filter_map(|r| {
            (r.kind == model::Kind::Tag)
                .then(|| semver::Version::parse(r.id.strip_prefix('v').unwrap_or(&r.id)).ok())
                .flatten()
                .map(|v| (v, r.instance, r))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, _, r)| r.clone())
}

/// The newest instance of a bare name, by precedence branch > tag > commit
/// (PROP-019 §2.3).
fn by_precedence_record(state: &State, name: &str) -> Option<InstallRecord> {
    for kind in [model::Kind::Branch, model::Kind::Tag, model::Kind::Commit] {
        if let Some(r) = state
            .installs
            .iter()
            .filter(|r| r.kind == kind && r.id == name)
            .max_by_key(|r| r.instance)
        {
            return Some(r.clone());
        }
    }
    None
}

/// The durable-env persister for this OS (PROP-019 §2.6): the registry on
/// Windows, the shell rc on POSIX.
fn make_persister(env: &VvmEnv, shell: env::Shell) -> Result<Box<dyn env::EnvPersister>, VvmError> {
    if cfg!(windows) {
        Ok(Box::new(env::WindowsEnvPersister))
    } else {
        let home = env.home.clone().ok_or(VvmError::NoHome)?;
        Ok(Box::new(env::RcFilePersister::new(
            shell.rc_path(&home),
            shell,
        )))
    }
}

/// Confirm a mutating action: `--yes`/unattended skip the prompt; a non-TTY
/// without `--yes` is an error rather than a silent apply.
fn confirm(ctx: &output::Context, yes: bool, prompt: &str) -> Result<bool, VvmError> {
    if yes || ctx.is_unattended() {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Err(VvmError::NoTty {
            detail: "no TTY for confirmation; pass `--yes` to proceed unattended".to_string(),
        });
    }
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(true)
        .interact()
        .unwrap_or(false))
}

/// Require an interactive TTY for a prompt that has no `--yes` bypass (the
/// remove / gc pickers): an unattended or non-TTY run errors with `msg` —
/// which names the explicit flags to pass — rather than silently doing
/// nothing (PROP-019 §2.9).
fn require_tty(ctx: &output::Context, msg: &str) -> Result<(), VvmError> {
    if ctx.is_unattended() || !std::io::stdin().is_terminal() {
        return Err(VvmError::NoTty {
            detail: msg.to_string(),
        });
    }
    Ok(())
}

/// Whether `dir` is on the `PATH` value, comparing canonicalised paths.
fn path_has_dir(path_var: Option<&str>, dir: &Path) -> bool {
    let Some(pv) = path_var else {
        return false;
    };
    let target = dir.canonicalize();
    std::env::split_paths(pv)
        .any(|p| p == dir || matches!((p.canonicalize(), &target), (Ok(a), Ok(b)) if &a == b))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
