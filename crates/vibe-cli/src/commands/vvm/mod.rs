//! `vibe self` — the VibeVM Version Manager (VVM): build, install, switch,
//! and remove vibevm's own versions on this machine (PROP-019). A
//! standalone-mode capability — pure algorithm, no LLM (PROP-019 §2.1).
//!
//! Dispatches every `vibe self` verb over the instance layout and the live
//! `current` pointer (PROP-019 §2.4, §2.5).

specmark::scope!("spec://vibevm/common/PROP-019#surface");

mod builder;
mod doctor;
mod embedded;
mod env;
mod error;
mod git;
mod install;
mod launchers;
mod model;
mod placer;
mod relocate;
mod remove;
pub(crate) mod selfloc;
mod source;
mod store;
mod tools;
mod vibeterm_packager;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::Result;
use dialoguer::Confirm;

use crate::cli::{
    ForcedKind, VvmArgs, VvmEnvArgs, VvmInstallArgs, VvmSubcommand, VvmUpdateArgs, VvmUseArgs,
};
use crate::output;

use error::VvmError;
use model::{InstallRecord, State, VersionId};
use store::VersionStore;

pub(crate) use embedded::embedded_root_at;
pub use selfloc::{derive_self, same_location};

/// Env var naming the install base (defaults to the user's home dir); the
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
    /// `~/opt`.
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

pub fn run(ctx: &output::Context, args: VvmArgs, env: VvmEnv) -> Result<()> {
    match args.command {
        VvmSubcommand::Install(a) => run_install_cmd(ctx, &env, a),
        VvmSubcommand::Update(a) => run_update_cmd(ctx, &env, a),
        VvmSubcommand::Use(a) => run_use_cmd(ctx, &env, a),
        VvmSubcommand::Ls => run_ls(ctx, &env),
        VvmSubcommand::Current => run_current(ctx, &env),
        VvmSubcommand::Which => run_which(ctx, &env),
        VvmSubcommand::Doctor(a) => doctor::run_doctor_cmd(ctx, &env, a),
        VvmSubcommand::Remove(a) => remove::run_remove_cmd(ctx, &env, a),
        VvmSubcommand::Gc(a) => remove::run_gc_cmd(ctx, &env, a),
        VvmSubcommand::Env(a) => run_env_cmd(&env, a),
        VvmSubcommand::Relocate(a) => relocate::run_relocate_cmd(ctx, &env, a),
    }
}

fn same_record(a: &InstallRecord, b: &InstallRecord) -> bool {
    a.version_id() == b.version_id() && a.instance == b.instance
}

fn run_ls(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let mut state = store.load_state()?;
    state
        .installs
        .sort_by(|a, b| a.id.cmp(&b.id).then(a.instance.cmp(&b.instance)));
    let active = store.active()?;

    if ctx.is_json() {
        let installs: Vec<serde_json::Value> = state
            .installs
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.version_id().to_string(),
                    "instance": r.instance,
                    "commit": r.commit,
                    "toolchain": r.toolchain,
                    "profile": r.profile.as_str(),
                    "origin": r.origin.as_str(),
                    "source_path": r.source_path,
                    "installed_at": r.installed_at,
                    "active": active.as_ref().map(|a| same_record(a, r)).unwrap_or(false),
                })
            })
            .collect();
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:ls",
            "active": active.as_ref().map(|a| a.version_id().to_string()),
            "count": installs.len(),
            "installs": installs,
        }));
    }

    if state.installs.is_empty() {
        ctx.summary("(no versions installed — run `vibe self install`)");
        return Ok(());
    }
    for r in &state.installs {
        let marker = if active.as_ref().map(|a| same_record(a, r)).unwrap_or(false) {
            "*"
        } else {
            " "
        };
        ctx.step(&format!(
            "{marker} {} #{}  {}  {}  {}",
            r.version_id(),
            r.instance,
            builder::short_commit(&r.commit),
            r.profile.as_str(),
            r.origin.as_str()
        ));
    }
    ctx.summary(&format!("{} instance(s) installed.", state.installs.len()));
    Ok(())
}

fn run_current(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let active = store.active()?;
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:current",
            "active": active.as_ref().map(|r| r.version_id().to_string()),
            "instance": active.as_ref().map(|r| r.instance),
        }));
    }
    match active {
        Some(r) => ctx.summary(&format!("{} #{}", r.version_id(), r.instance)),
        None => ctx.summary("(no active version)"),
    }
    Ok(())
}

fn run_which(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let Some(record) = store.active()? else {
        return Err(VvmError::NoActiveVersion.into());
    };
    let path = store.binary_path(&record.version_id(), record.instance);
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:which",
            "path": path.display().to_string(),
        }));
    }
    ctx.summary(&path.display().to_string());
    Ok(())
}

fn run_install_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmInstallArgs) -> Result<()> {
    let store = env.store()?;
    let profile = resolve_profile(&args)?;
    let selector = model::Selector::parse(&args.selector, forced_kind(&args.kind))?;
    let now = chrono::Utc::now().to_rfc3339();

    // Three source origins (PROP-019 §2.7, §2.16):
    //   (a) in-tree   — the committer's own checkout, built in place;
    //   (b) linked    — rebuild an external version from its remembered path,
    //                   without being in the checkout;
    //   (c) managed   — fetch/clone the mirror and build from it.
    let in_tree = env.cwd.as_deref().and_then(source::find_source_root);
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
    install::perform_install(
        ctx,
        &store,
        &source_dir,
        &req,
        &builder::CargoBuilder,
        &vibeterm_packager::NpmPackager::new(ctx),
        &launchers::NativeLauncherInstaller,
    )
}

/// `self update` — rebuild and activate the latest in-tree version. A thin
/// shorthand over `self install latest` (PROP-019 §2.2) that fixes the
/// selector to `latest` and carries only the build knobs.
fn run_update_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmUpdateArgs) -> Result<()> {
    run_install_cmd(
        ctx,
        env,
        VvmInstallArgs {
            selector: "latest".to_string(),
            kind: ForcedKind {
                tag: false,
                branch: false,
                commit: false,
            },
            profile: args.profile,
            release: args.release,
            mirror: None,
            force: args.force,
        },
    )
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
    let state = store.load_state()?;
    let selector = model::Selector::parse(&args.selector, forced_kind(&args.kind))?;
    let rec = resolve_installed(&state, &selector, &args.selector)?;
    let id = rec.version_id();
    let home = store.instance_dir(&id, rec.instance);
    let shell = env::Shell::detect(env.shell.as_deref());

    if args.eval {
        // Print only the line to eval in the current shell; persist nothing.
        println!("{}", shell.export_line(&home));
        return Ok(());
    }

    // Flip the live pointer — the switch is instant, no console reload.
    store.write_current(&home)?;
    // Keep shims present and the advisory env current for external tools.
    env::write_shims(&store.shim_dir())?;
    let persister = make_persister(env, shell)?;
    persister.set_vibevm_home(&home)?;
    persister.ensure_on_path(&store.shim_dir())?;

    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "self:use",
            "active": id.to_string(),
            "instance": rec.instance,
            "home": home.display().to_string(),
        }));
    }
    ctx.summary(&format!("active → {id} #{}", rec.instance));
    ctx.summary("  switched live; the next `vibe` in this shell uses it");
    ctx.summary(&format!(
        "  external tools: {}",
        persister.activation_hint()
    ));
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
            store.instance_dir(&rec.version_id(), rec.instance)
        }
        None => {
            let rec = store.active()?.ok_or(VvmError::NoActiveVersion)?;
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
