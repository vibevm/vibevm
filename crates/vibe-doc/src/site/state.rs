//! The builder's state — what is rendered, and what it was rendered from
//! (PROP-057 `##SITE-RENDER-KEY`, campaign atom A5.2).
//!
//! One file, `<out>/.vibe-site/state.json`, beside the render it
//! describes. Beside it and not in some machine-wide place, because the
//! question it answers is «what is in THIS directory» — point a second
//! run at a fresh directory and the honest answer is «nothing», which is
//! what an absent file already says.
//!
//! ## It is a key, not a memory
//!
//! There is exactly one row per coordinate and version, holding the
//! current render. A version number may be published many times and only
//! the last publication exists — in the registry as on the site
//! (`##SITE-VERSION-SHOWS-CURRENT`) — so a second row for an older
//! publication would describe something that is not there. Nothing here
//! is ever shown to a reader: the content hash is the builder's key, and
//! a page that displayed it would be claiming to remember a past this
//! project deliberately does not keep (§14).
//!
//! ## Why a failed render is a row and not an absence
//!
//! A render that failed leaves a «render failed» page at the address
//! (`##SITE-RENDER-IDEMPOTENT`), and the row says so. It has to: a failed
//! package whose source never moves again would otherwise never be tried
//! again, because nothing about it would ever differ from what was
//! rendered. Recording the failure is what makes the retry automatic.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-RENDER-KEY");

use std::path::{Path, PathBuf};

use vibe_wire::generated::doc_site_state::{DocSiteState, RenderedVersion};

use crate::error::{DocError, Result};

/// The schema number a state file written today carries.
pub const SCHEMA_VERSION: u32 = 1;

/// The builder's own directory inside the output. Dotted so it sorts and
/// reads as kitchen rather than as content.
///
/// It is INSIDE the output on purpose: the state describes this render,
/// travels with it, and a second run against the same directory is the
/// only thing that makes the build incremental. The price is that a
/// serving configuration has to refuse it — the builder's key is not a
/// file the domain publishes — and that belongs in the serving config,
/// where every other «do not serve this» rule already lives.
pub const STATE_DIR: &str = ".vibe-site";

/// The state file's name inside [`STATE_DIR`].
pub const STATE_FILE: &str = "state.json";

/// Where the state file sits under an output directory.
pub fn path(out_dir: &Path) -> PathBuf {
    out_dir.join(STATE_DIR).join(STATE_FILE)
}

/// Read the state of an output directory, or the empty state when it has
/// none.
///
/// An absent file is not an error and never will be: it is what a fresh
/// output directory says, and it says it correctly — nothing is rendered
/// there.
///
/// ```
/// let tmp = tempfile::tempdir().unwrap();
/// let state = vibe_doc::site::state::read(tmp.path()).unwrap();
/// assert!(state.rendered.is_empty());
/// assert!(state.host_rendered_at.is_none());
/// ```
pub fn read(out_dir: &Path) -> Result<DocSiteState> {
    let file = path(out_dir);
    if !file.is_file() {
        return Ok(empty());
    }
    let text = std::fs::read_to_string(&file).map_err(|e| DocError::io("reading", &file, e))?;
    let state: DocSiteState = serde_json::from_str(&text).map_err(|e| DocError::Site {
        message: format!("`{}` is not a builder state file: {e}", file.display()),
    })?;
    if state.schema != SCHEMA_VERSION {
        return Err(DocError::Site {
            message: format!(
                "`{}` declares `schema` {}, and this build reads {SCHEMA_VERSION} — \
                 delete the file to render everything again",
                file.display(),
                state.schema
            ),
        });
    }
    Ok(state)
}

/// The state of a directory nothing has been rendered into.
pub fn empty() -> DocSiteState {
    DocSiteState {
        schema: SCHEMA_VERSION,
        rendered: Vec::new(),
        host_rendered_at: None,
    }
}

/// Write the state, sorting the rows so two runs over an unchanged site
/// produce the same bytes.
///
/// The sort is not cosmetic: a state file whose order followed whichever
/// package finished first would differ between two identical runs, and
/// «the same sources render to the same bytes» is the property that makes
/// a deploy diffable.
pub fn write(out_dir: &Path, state: &DocSiteState) -> Result<()> {
    let file = path(out_dir);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
    }
    let mut ordered = state.clone();
    ordered.rendered.sort_by_key(key);
    let mut text = serde_json::to_string_pretty(&ordered).map_err(|e| DocError::Site {
        message: format!("the builder state will not serialise: {e}"),
    })?;
    text.push('\n');
    std::fs::write(&file, text).map_err(|e| DocError::io("writing", &file, e))
}

/// What a row is keyed by: the coordinate and the version, and never the
/// source. Two registries offering one coordinate is a collision to
/// report, not two rows to keep.
pub fn key(row: &RenderedVersion) -> (String, String, String) {
    (
        row.group.to_string(),
        row.name.clone(),
        row.version.to_string(),
    )
}

/// The row for one coordinate and version, when there is one.
pub fn rendered<'a>(
    state: &'a DocSiteState,
    group: &str,
    name: &str,
    version: &str,
) -> Option<&'a RenderedVersion> {
    let wanted = (group.to_string(), name.to_string(), version.to_string());
    state.rendered.iter().find(|row| key(row) == wanted)
}

#[cfg(test)]
mod tests;
