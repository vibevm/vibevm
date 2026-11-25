//! Read-only px presence and provenance, shared by both status aliases.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools");

use anyhow::Result;

use crate::output::language::{Block, Glyph, LabelValue, Line, viewport, wrap};

use super::{PxVerdict, VvmEnv, companions, output, unusable_verdict};

pub(super) fn run(ctx: &output::Context, env: &VvmEnv) -> Result<PxVerdict> {
    let store = env.store()?;
    let pin = vibe_net::px_store::PxPin::embedded();
    let root = companions::store_root(&store);
    let asset = companions::pinned_asset(pin);
    let installed = companions::installed(&root);
    let name = asset.map(|asset| pin.release_asset_name(asset));
    let directory = vibe_net::px_store::stored_dir(&root, pin);
    // What the route would actually run, by the same lookup every
    // operation uses: `px_path`, then the store, then `tools/px/`
    // beside the binary, then PATH. An empty store is a problem
    // only when px mode has nothing else to run.
    let route = vibe_core::net::route();
    let needed = route.px().is_some();
    let program = route
        .px()
        .and_then(|settings| vibe_net::locate_px(settings).ok());
    let usable = !needed || program.is_some();
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "version": pin.version,
            "asset": name,
            "store": directory,
            "installed": installed.as_ref().map(|found| &found.program),
            "in_store": installed.is_some(),
            "needed": needed,
            "program": program,
            "source": installed.as_ref().and_then(|found| found.source.clone()),
            "sha256": installed.as_ref().and_then(|found| found.sha256.clone()),
            "installed_at": installed
                .as_ref()
                .and_then(|found| found.installed_at.clone()),
            "px_version": installed.as_ref().and_then(|found| found.version.clone()),
            "ok": usable,
        }))?;
        return Ok(unusable_verdict(usable, pin, &directory, name.as_deref()));
    }
    let mut block = if let Some(found) = &installed {
        Block::new(
            Glyph::Done,
            format!("px  installed · {}", found.program.display()),
        )
    } else {
        Block::new(
            Glyph::Skipped,
            format!("px  not installed · px {} · {}", pin.version, root.display()),
        )
    };
    if let Some(found) = &installed {
        for (label, value) in [
            ("reports", &found.version),
            ("read from", &found.source),
            ("sha256", &found.sha256),
            ("installed", &found.installed_at),
        ] {
            if let Some(value) = value {
                block.body.push(Line::Label(
                    label.into(),
                    vec![LabelValue::Text(value.clone())],
                ));
            }
        }
    }
    let mut report = block.render();
    let mut rows = Vec::new();
    if installed.is_none() {
        if let Some(name) = &name {
            rows.push(format!("asset  {name}"));
            rows.push("install from the release source or drop, or from your own archive:".into());
            rows.push("vibe self px install".into());
        } else {
            rows.push("no default build for this platform; install from your own archive:".into());
        }
        rows.push("vibe self px install --from <archive|directory|executable>".into());
    }
    match (&installed, &program) {
        (None, Some(program)) => rows.push(format!(
            "[proxy] mode = \"px\" runs {} instead",
            program.display()
        )),
        (None, None) if needed => rows.push(
            "[proxy] mode = \"px\" has no px to run: the next network command will fail".into(),
        ),
        _ => {}
    }
    for row in rows {
        for line in wrap(&row, viewport().saturating_sub(2)) {
            report.push_str("\n  ");
            report.push_str(&line);
        }
    }
    // Status is requested data, including under --quiet; it has no trailer.
    ctx.summary(&report);
    Ok(unusable_verdict(usable, pin, &directory, name.as_deref()))
}
