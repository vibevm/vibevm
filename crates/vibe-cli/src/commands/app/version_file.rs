//! `--write-version-file` on `vibe app use` and `vibe install -g`
//! (PROP-075 `#VERSION-FILE-WRITE`), which is nvm's `--save` / `-w`.
//!
//! The library decides how a file is written and which file a bare flag
//! lands in; this module's whole job is supplying the facts it cannot know
//! — the slot's declared rows and the resolved version, read from the
//! index record of the installation the command just settled on, and the
//! nearest file the ordinary version-file search finds from here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE");

use anyhow::{Context, Result};
use vibe_app::{DeclaredVersionFiles, Index, SettingsRoot, WriteRequest, WrittenVersionFile};
use vibe_wire::generated::application_alternatives::VersionFile as AlternativesWire;
use vibe_wire::generated::application_report::VersionFile as ReportWire;

use crate::cli::WriteVersionFileArgs;
use crate::output;

/// Write the resolved version of `key` into the slot's declared version
/// file, or do nothing when the flag was not given.
///
/// The version written is the **installed** one — read off the index record
/// — never the requirement or the channel the user typed, because a version
/// file that says `lts` tells the next reader nothing about what was
/// chosen.
pub(super) fn write(
    settings: &SettingsRoot,
    args: &WriteVersionFileArgs,
    slot: &str,
    key: &str,
) -> Result<Option<WrittenVersionFile>> {
    if !args.asked() {
        return Ok(None);
    }
    let index = Index::load(settings)?;
    let installed = index
        .installations
        .get(key)
        .with_context(|| format!("installation `{key}` is not recorded in this home"))?;
    let package = format!("{}/{}", installed.package.group, installed.package.name);
    let declared = DeclaredVersionFiles::load(settings);
    let rows = declared.for_slot(slot);
    let cwd = std::env::current_dir().context("resolving the working directory")?;
    let project = super::env::enclosing_project(&cwd);
    // The nearest file the `#VERSION-FILE-FORMATS` search already finds, so
    // a bare flag edits the file the project is actually using.
    let search = vibe_app::resolve_version_files(settings, &cwd, project.as_deref(), &declared);
    let found = search.of_slot(slot).map(|found| found.path.clone());
    let written = vibe_app::write_version_file(&WriteRequest {
        slot,
        package: &package,
        version: &installed.package.version,
        rows: &rows,
        path: args.path(),
        found: found.as_deref(),
        create_in: project.as_deref().unwrap_or(&cwd),
    })
    .map_err(vibe_app::AppError::from)?;
    // The resolution of this directory now has a different answer.
    vibe_app::clear_version_file_cache(settings);
    Ok(Some(written))
}

/// The written file as the install report carries it (`AV-FMT-019`).
pub(crate) fn wire_report(written: &WrittenVersionFile) -> ReportWire {
    ReportWire {
        path: vibe_core::machine_json_path(&written.path),
        version: written.version.clone(),
    }
}

/// The written file as `vibe app use --json`'s listing carries it.
pub(crate) fn wire_alternatives(written: &WrittenVersionFile) -> AlternativesWire {
    AlternativesWire {
        path: vibe_core::machine_json_path(&written.path),
        version: written.version.clone(),
    }
}

/// The one human line: the path and the version written.
pub(crate) fn report(ctx: &output::Context, command: &str, written: &WrittenVersionFile) {
    ctx.summary(&format!(
        "{command}: wrote {} to {}",
        written.version,
        written.path.display()
    ));
}
