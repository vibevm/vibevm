//! Verification and activation for already-installed VibeVM instances.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#activation");

use super::*;
use vibe_core::progress::ProgressTask;

pub(super) fn staged<T>(
    ctx: &output::Context,
    label: &str,
    run: impl FnOnce(&ProgressTask) -> Result<T>,
) -> Result<T> {
    let task = ctx.progress().task(label);
    match run(&task) {
        Ok(value) => {
            task.finish();
            Ok(value)
        }
        Err(error) => {
            task.fail(error.to_string());
            Err(error)
        }
    }
}

pub(super) fn run_use_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmUseArgs) -> Result<()> {
    let store = env.store()?;
    let selector = model::Selector::parse(&args.selector, forced_kind(&args.kind))?;

    if args.eval {
        let record = staged(ctx, "Verifying selected version", |task| {
            task.detail(format!("selector: {}", args.selector));
            let state = store.load_state()?;
            let record = resolve_installed(&state, &selector, &args.selector)?;
            ensure_activatable(&store, &record)?;
            Ok(record)
        })?;
        let home = store.instance_dir(&record.version_id(), record.instance);
        let shell = env::Shell::detect(env.shell.as_deref());
        ctx.suspend_progress(|| println!("{}", shell.export_line(&home)));
        return Ok(());
    }

    let _lock = install::InstallLock::acquire(&store)?;
    let record = staged(ctx, "Verifying selected version", |task| {
        task.detail(format!("selector: {}", args.selector));
        let state = store.load_state()?;
        let record = resolve_installed(&state, &selector, &args.selector)?;
        ensure_activatable(&store, &record)?;
        Ok(record)
    })?;
    staged(ctx, "Activating selected version", |task| {
        task.detail(format!("selector: {}", record.selector()));
        activate_record(ctx, env, &store, &record, "self:use")
    })
}

pub(super) fn run_rollback_cmd(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let store = env.store()?;
    let _lock = install::InstallLock::acquire(&store)?;
    let record = staged(ctx, "Verifying rollback version", |task| {
        let record = store.previous()?.ok_or(VvmError::NoRollback)?;
        task.detail(format!("selector: {}", record.selector()));
        ensure_activatable(&store, &record).map_err(|_| VvmError::NoRollback)?;
        Ok(record)
    })?;
    staged(ctx, "Activating rollback version", |task| {
        task.detail(format!("selector: {}", record.selector()));
        activate_record(ctx, env, &store, &record, "self:rollback")
    })
}

pub(super) fn ensure_activatable(
    store: &VersionStore,
    record: &InstallRecord,
) -> Result<(), VvmError> {
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

pub(super) fn activate_record(
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

pub(super) fn run_env_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmEnvArgs) -> Result<()> {
    let (shell, home) = staged(ctx, "Resolving shell environment", |task| {
        let shell = match args.shell.as_deref() {
            Some(value) => env::Shell::parse(value)?,
            None => env::Shell::detect(env.shell.as_deref()),
        };
        let store = env.store()?;
        let home = match args.selector.as_deref() {
            Some(raw) => {
                task.detail(format!("selector: {raw}"));
                let state = store.load_state()?;
                let selector = model::Selector::parse(raw, forced_kind(&args.kind))?;
                let record = resolve_installed(&state, &selector, raw)?;
                ensure_activatable(&store, &record)?;
                store.instance_dir(&record.version_id(), record.instance)
            }
            None => {
                let record = store.active()?.ok_or(VvmError::NoActiveVersion)?;
                task.detail(format!("selector: {}", record.selector()));
                ensure_activatable(&store, &record)?;
                store.instance_dir(&record.version_id(), record.instance)
            }
        };
        Ok((shell, home))
    })?;
    ctx.suspend_progress(|| println!("{}", shell.export_line(&home)));
    Ok(())
}
