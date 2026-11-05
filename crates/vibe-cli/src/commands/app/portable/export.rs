//! Export output is fully prepared before opening the destination.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
use crate::{cli::AppExportArgs, output};
use anyhow::{Context, Result};
use std::io::Write;
use vibe_app::{PortableDocument, PortableExport, SettingsRoot};
/// `vibe app export`: the set as TOML on stdout or in `--output`, or as
/// JSON with `--json`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub(super) fn run(ctx: &output::Context, home: &SettingsRoot, args: &AppExportArgs) -> Result<()> {
    let request = PortableExport {
        all: args.all,
        arrangement: !args.installations_only,
    };
    let set = vibe_app::export_portable(home, &request, args.configuration)?;
    let text = if ctx.is_json() {
        set.json()?
    } else {
        set.toml()?
    };
    if let Some(path) = &args.output {
        std::fs::write(path, &text).with_context(|| format!("writing {}", path.display()))?;
    }
    if ctx.is_json() {
        match &set {
            PortableDocument::Legacy(legacy) => return ctx.emit_json(legacy),
            // Full stdout is the portable document itself: command-context
            // stamps would be unknown members of its closed reader.
            PortableDocument::Full(_) => std::io::stdout()
                .lock()
                .write_all(text.as_bytes())
                .context("writing the portable document to stdout")?,
        }
    } else if let Some(path) = &args.output {
        ctx.trailer(&format!(
            "exported {} application(s) · {}",
            set.application_count(),
            path.display()
        ));
    } else {
        std::io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .context("writing the portable document to stdout")?;
    }
    Ok(())
}
