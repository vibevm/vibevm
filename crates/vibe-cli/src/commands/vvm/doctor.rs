//! `vibe self doctor` — the environment health check (PROP-019 §2.8), split out
//! of the `vvm` hub along the command-handler seam to keep the hub under the
//! 600-line file budget.
//!
//! The terminal apps (vibeterm, vibeframe) and the GUI launchers used to be
//! packaged into the instance alongside `vibe`; they have moved to a separate
//! products repo and now publish themselves to `PATH`. The doctor no longer
//! probes for them — `vibe term` / `vibe frame` resolve through `PATH` (with
//! an in-place fallback for `vibe tree`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use crate::cli::VvmDoctorArgs;
use crate::output;

use super::embedded;
use super::env;
use super::model::Origin;
use super::tools;
use super::{VvmEnv, confirm, current_record, make_persister, path_has_dir};

fn needs_build_tools(origin: Option<Origin>) -> bool {
    origin != Some(Origin::Binary)
}

/// `vibe self doctor`: probe the required toolchain, the shim dir on PATH,
/// the active version's binary, and the embedded registry (PROP-030).
/// Problems block.
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
    let current = current_record(&store)?;
    let build_tools_required = needs_build_tools(current.as_ref().map(|record| record.origin));
    let embedded_registry = current.as_ref().and_then(embedded::embedded_root_for);
    let active_missing = active
        .as_ref()
        .map(|r| !store.binary_path(&r.version_id(), r.instance).is_file())
        .unwrap_or(false);
    let problems = usize::from(build_tools_required) * tools.iter().filter(|t| !t.ok).count()
        + usize::from(!on_path)
        + usize::from(active_missing);

    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": problems == 0,
            "command": "self:doctor",
            "problems": problems,
            "tools": tools.iter().map(|t| serde_json::json!({
                "name": t.name, "version": t.version, "ok": t.ok,
                "required": build_tools_required,
                "min": t.min_version, "help": t.help_url,
            })).collect::<Vec<_>>(),
            "shim_dir": shim_dir.display().to_string(),
            "shim_dir_on_path": on_path,
            "active": active.as_ref().map(|r| r.version_id().to_string()),
            "current": current.as_ref().map(|r| r.selector().to_string()),
            "build_tools_required": build_tools_required,
            "active_binary_ok": !active_missing,
            "embedded_registry": embedded_registry.as_ref().map(|p| p.display().to_string()),
        }));
    }

    ctx.heading("vibe self doctor");
    for t in &tools {
        match (&t.version, build_tools_required) {
            (Some(v), false) => ctx.step(&format!(
                "info {} {} (optional for binary instance)",
                t.name, v
            )),
            (None, false) => ctx.step(&format!(
                "skip {} not found (not required by binary instance)",
                t.name
            )),
            (Some(v), true) if t.ok => ctx.step(&format!("ok   {} {}", t.name, v)),
            (Some(v), true) => ctx.step(&format!(
                "MISS {} {} (need >= {}) — {}",
                t.name, v, t.min_version, t.help_url
            )),
            (None, true) => ctx.step(&format!("MISS {} not found — {}", t.name, t.help_url)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_instances_do_not_require_a_source_build_toolchain() {
        assert!(!needs_build_tools(Some(Origin::Binary)));
        assert!(needs_build_tools(Some(Origin::External)));
        assert!(needs_build_tools(Some(Origin::Managed)));
        assert!(needs_build_tools(None));
    }
}
