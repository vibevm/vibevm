//! `vibe self doctor` — the environment health check (PROP-019 §2.8), split out
//! of the `vvm` hub along the command-handler seam to keep the hub under the
//! 600-line file budget. Carries the vibeterm-packaging advisory (§2.7).

specmark::scope!("spec://vibevm/common/PROP-019#surface");

use crate::cli::VvmDoctorArgs;
use crate::output;

use super::embedded;
use super::env;
use super::tools;
use super::{VvmEnv, confirm, make_persister, path_has_dir};

/// The executable name a packaged vibeterm ships — electron-packager names it
/// after the app (`vibeterm`), not `electron` (see `term::electron_binary`).
fn packaged_vibeterm_exe() -> &'static str {
    if cfg!(windows) {
        "vibeterm.exe"
    } else {
        "vibeterm"
    }
}

/// `vibe self doctor`: probe the required toolchain, the shim dir on PATH, the
/// active version's binary, the embedded registry (PROP-030), and — advisorially
/// — the optional node/npm that package vibeterm and whether the active instance
/// actually carries a packaged vibeterm (PROP-019 §2.7). Problems block; the
/// vibeterm rows never do (a Rust-only box is healthy without it).
pub(super) fn run_doctor_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmDoctorArgs,
) -> anyhow::Result<()> {
    let store = env.store()?;
    let tools = tools::check_all();
    let shim_dir = store.shim_dir();
    let on_path = path_has_dir(env.path_var.as_deref(), &shim_dir);
    let active = store.active()?;
    let embedded_registry = active.as_ref().and_then(embedded::embedded_root_for);
    let active_missing = active
        .as_ref()
        .map(|r| !store.binary_path(&r.version_id(), r.instance).is_file())
        .unwrap_or(false);
    let problems = tools.iter().filter(|t| !t.ok).count()
        + usize::from(!on_path)
        + usize::from(active_missing);

    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": problems == 0,
            "command": "self:doctor",
            "problems": problems,
            "tools": tools.iter().map(|t| serde_json::json!({
                "name": t.name, "version": t.version, "ok": t.ok,
                "min": t.min_version, "help": t.help_url,
            })).collect::<Vec<_>>(),
            "shim_dir": shim_dir.display().to_string(),
            "shim_dir_on_path": on_path,
            "active": active.as_ref().map(|r| r.version_id().to_string()),
            "active_binary_ok": !active_missing,
            "embedded_registry": embedded_registry.as_ref().map(|p| p.display().to_string()),
            "optional_tools": tools::check_optional().iter().map(|t| serde_json::json!({
                "name": t.name, "version": t.version, "ok": t.ok, "min": t.min_version,
            })).collect::<Vec<_>>(),
            "vibeterm_packaged": active.as_ref().filter(|_| !active_missing).is_some_and(|r| {
                store.instance_dir(&r.version_id(), r.instance)
                    .join("vibeterm").join(packaged_vibeterm_exe()).is_file()
            }),
        }));
    }

    ctx.heading("vibe self doctor");
    for t in &tools {
        match &t.version {
            Some(v) if t.ok => ctx.step(&format!("ok   {} {}", t.name, v)),
            Some(v) => ctx.step(&format!(
                "MISS {} {} (need >= {}) — {}",
                t.name, v, t.min_version, t.help_url
            )),
            None => ctx.step(&format!("MISS {} not found — {}", t.name, t.help_url)),
        }
    }
    let (linker, lurl) = tools::linker_hint();
    ctx.step(&format!("also {linker} — {lurl}"));
    ctx.step(&format!(
        "{} shim dir {} {}",
        if on_path { "ok  " } else { "MISS" },
        shim_dir.display(),
        if on_path {
            "(on PATH)"
        } else {
            "(NOT on PATH)"
        }
    ));
    match &active {
        Some(r) if !active_missing => {
            ctx.step(&format!("ok   active {} #{}", r.version_id(), r.instance))
        }
        Some(r) => ctx.step(&format!(
            "MISS active {} — its binary is gone",
            r.version_id()
        )),
        None => ctx.step("-    no active version (set one with `vibe self use <selector>`)"),
    }
    // PROP-030: the embedded registry the active source install exposes for
    // every project (its in-tree `packages/`).
    match &embedded_registry {
        Some(root) => ctx.step(&format!(
            "ok   embedded registry {} (source install; precedence embedded-first)",
            root.display()
        )),
        None => ctx.step("-    no embedded registry (the active version is not a source install)"),
    }
    // Advisory (non-blocking): the optional tools that package vibeterm, and
    // whether the active instance actually carries a packaged vibeterm.
    for t in tools::check_optional() {
        match &t.version {
            Some(v) if t.ok => ctx.step(&format!(
                "ok   {} {} (optional: vibeterm packaging)",
                t.name, v
            )),
            Some(v) => ctx.step(&format!(
                "old  {} {} (need >= {}; optional) — {}",
                t.name, v, t.min_version, t.help_url
            )),
            None => ctx.step(&format!(
                "-    {} not found (optional: vibeterm packaging) — {}",
                t.name, t.help_url
            )),
        }
    }
    if let Some(r) = active.as_ref().filter(|_| !active_missing) {
        let packaged = store
            .instance_dir(&r.version_id(), r.instance)
            .join("vibeterm")
            .join(packaged_vibeterm_exe())
            .is_file();
        ctx.step(if packaged {
            "ok   vibeterm packaged alongside the active version"
        } else {
            "-    vibeterm not packaged in the active instance (`vibe term` will name the setup step)"
        });
    }

    if args.fix && confirm(ctx, args.yes, "Write shims and put the shim dir on PATH?")? {
        env::write_shims(&shim_dir)?;
        let shell = env::Shell::detect(env.shell.as_deref());
        make_persister(env, shell)?.ensure_on_path(&shim_dir)?;
        ctx.summary("fixed: shims written, shim dir ensured on PATH (open a new shell).");
    }

    if problems == 0 {
        ctx.summary("all good.");
    } else {
        ctx.summary(&format!("{problems} problem(s) — see above."));
    }
    Ok(())
}
