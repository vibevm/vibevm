//! TOML text in, TOML text out: the pure render of PROP-075
//! `#TEMPLATE-RENDER-LOCATION`.
//!
//! The template is parsed with `toml_edit`, so an author's formatting,
//! key order and comments survive the substitution untouched, and the
//! result is read by the ordinary `Manifest::parse_str` — which is what
//! makes `#TEMPLATE-VARIABLES`' "an ordinary manifest validated by the
//! ordinary rules" literally true.
//!
//! The render also **rewrites the identity**: `[package].version` becomes
//! the produced version (the template's own `version` is the revision of
//! the template), and `[package].template`, the whole `[template]` table
//! and every `template_optional` key are removed, so the rendered document
//! carries no template vocabulary at all.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-RENDER-LOCATION");

use sha2::{Digest, Sha256};
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::error::{RenderError, RenderErrorKind};
use crate::source::{DigestTable, VersionRecord, template_section};

mod grammar;
mod substitute;

use substitute::{Context, substitute_table};

/// The digest a **skeleton** render writes into every digest-bearing
/// field: a declared placeholder, deliberately a valid 64-digit digest of
/// nothing but zeros so the skeleton is read by the ordinary manifest
/// rules without an exemption, and deliberately impossible to mistake for
/// a real one. A skeleton is never installed from, never written into a
/// store and never hashed into a lock (PROP-075 `#TEMPLATE-TWO-STAGE`).
pub const SKELETON_DIGEST: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Which of the two stages produced a rendering.
///
/// ```
/// use vibe_template::Stage;
///
/// // Only a full render's bytes may be hashed into a lock.
/// assert_ne!(Stage::Skeleton, Stage::Full);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The version record alone; digest-bearing fields hold
    /// [`SKELETON_DIGEST`].
    Skeleton,
    /// The digests substituted — the only stage whose bytes may be hashed
    /// into a lock or materialised into a store.
    Full,
}

/// One `[[installer_options]]` row the render dropped because a digest
/// variable it references has no value for this version and the row
/// declared `template_optional` (PROP-075 `#TEMPLATE-OPTION-GATING`).
///
/// ```
/// use vibe_template::DroppedOption;
///
/// let row = DroppedOption {
///     id: "linux-musl-x64".to_string(),
///     missing: "node-v24.22.2-linux-x64-musl.tar.gz".to_string(),
/// };
/// assert_eq!(row.id, "linux-musl-x64");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedOption {
    /// The row's `id`.
    pub id: String,
    /// The file the digest document does not name.
    pub missing: String,
}

/// What a render produced.
///
/// ```
/// use vibe_template::{DigestTable, VersionRecord, render};
///
/// let template = concat!(
///     "[package]\nname = \"t\"\ngroup = \"org.e\"\nkind = \"app\"\nversion = \"1.0.0\"\n",
///     "template = true\n\n[application]\nid = \"t\"\n\n",
///     "[template.versions]\nkind = \"list\"\n\n",
///     "[[template.versions.item]]\nversion = \"2.0.0\"\n\n",
///     "[template.digests]\nkind = \"sha256sums\"\nurl = \"https://e.invalid/{version}\"\n",
/// );
/// let out = render(template, &VersionRecord::plain("2.0.0")?, &DigestTable::default())?;
/// assert_eq!(out.sha256.len(), 64);
/// assert!(out.dropped.is_empty());
/// assert!(out.toml.starts_with("[package]"));
/// # Ok::<(), vibe_template::RenderError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The rendered manifest's TOML text.
    pub toml: String,
    /// The SHA-256 of those bytes — the `rendered_sha256` of PROP-075
    /// `#TEMPLATE-LOCK-RECORD` when the stage is [`Stage::Full`].
    pub sha256: String,
    /// The rows `template_optional` dropped, in declaration order.
    pub dropped: Vec<DroppedOption>,
    pub stage: Stage,
}

/// The **full** render of one version: every digest substituted.
///
/// ```
/// use vibe_template::{DigestTable, Stage, VersionRecord, render};
///
/// let template = concat!(
///     "[package]\nname = \"t\"\ngroup = \"org.e\"\nkind = \"app\"\nversion = \"1.0.0\"\n",
///     "template = true\n\n[application]\nid = \"t\"\n\n",
///     "[template.versions]\nkind = \"list\"\n\n",
///     "[[template.versions.item]]\nversion = \"2.0.0\"\n\n",
///     "[template.digests]\nkind = \"sha256sums\"\nurl = \"https://e.invalid/{version}\"\n",
/// );
/// let record = VersionRecord::plain("2.0.0")?;
/// let rendered = render(template, &record, &DigestTable::default())?;
/// assert_eq!(rendered.stage, Stage::Full);
/// assert!(rendered.toml.contains("version = \"2.0.0\""));
/// assert_eq!(rendered.sha256.len(), 64);
/// # Ok::<(), vibe_template::RenderError>(())
/// ```
pub fn render(
    template_toml: &str,
    record: &VersionRecord,
    digests: &DigestTable,
) -> Result<Rendered, RenderError> {
    render_stage(template_toml, record, Some(digests))
}

/// The **skeleton** render of one version: the version record alone, with
/// every digest-bearing field at [`SKELETON_DIGEST`]. Enough for
/// everything resolution needs, and one HTTP request cheaper.
///
/// ```
/// use vibe_template::{SKELETON_DIGEST, Stage, VersionRecord, skeleton};
///
/// let template = concat!(
///     "[package]\nname = \"t\"\ngroup = \"org.e\"\nkind = \"app\"\nversion = \"1.0.0\"\n",
///     "template = true\n\n[application]\nid = \"t\"\n\n",
///     "[template.versions]\nkind = \"list\"\n\n",
///     "[[template.versions.item]]\nversion = \"2.0.0\"\n\n",
///     "[template.digests]\nkind = \"sha256sums\"\nurl = \"https://e.invalid/{version}\"\n\n",
///     "[[installer_options]]\nid = \"l\"\nname = \"t\"\nplatform = \"linux\"\n",
///     "platform_name = \"Linux\"\narch = \"x86_64\"\npath_dirs = [\"bin\"]\n\n",
///     "[[installer_options.steps]]\nid = \"u\"\ntitle = \"u\"\nkind = \"archive\"\n",
///     "format = \"tar_gz\"\nsource = \"https://e.invalid/t-{version}.tar.gz\"\n",
///     "sha256 = \"{sha256:t-{version}.tar.gz}\"\n",
/// );
/// let rendered = skeleton(template, &VersionRecord::plain("2.0.0")?)?;
/// assert_eq!(rendered.stage, Stage::Skeleton);
/// assert!(rendered.toml.contains(SKELETON_DIGEST));
/// # Ok::<(), vibe_template::RenderError>(())
/// ```
pub fn skeleton(template_toml: &str, record: &VersionRecord) -> Result<Rendered, RenderError> {
    render_stage(template_toml, record, None)
}

fn render_stage(
    template_toml: &str,
    record: &VersionRecord,
    digests: Option<&DigestTable>,
) -> Result<Rendered, RenderError> {
    let section = template_section(template_toml)?;
    let mut document = template_toml.parse::<DocumentMut>().map_err(|error| {
        RenderError::template(format!("the template manifest is not TOML: {error}"))
    })?;
    let root = document.as_table_mut();
    root.remove("template");
    rewrite_identity(root, record)?;
    let context = Context {
        record,
        digests,
        section: &section,
        platform: None,
        arch: None,
    };
    substitute_outside_options(root, &context)?;
    let dropped = render_options(root, &context)?;
    let toml = document.to_string();
    Ok(Rendered {
        sha256: hex(Sha256::digest(toml.as_bytes()).as_slice()),
        toml,
        dropped,
        stage: if digests.is_some() {
            Stage::Full
        } else {
            Stage::Skeleton
        },
    })
}

/// `[package].version` becomes the produced version and the flag goes.
fn rewrite_identity(root: &mut Table, record: &VersionRecord) -> Result<(), RenderError> {
    let package = root
        .get_mut("package")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| {
            RenderError::template(
                "the template manifest carries no [package] table — a template is a package"
                    .to_string(),
            )
        })?;
    package.remove("template");
    let item = package.get_mut("version").ok_or_else(|| {
        RenderError::template(
            "the template manifest's [package] carries no `version` — the template's own version \
             is the revision of the template"
                .to_string(),
        )
    })?;
    let decor = item
        .as_value()
        .map(|value| value.decor().clone())
        .unwrap_or_default();
    let mut value = Value::from(record.version.to_string());
    *value.decor_mut() = decor;
    *item = Item::Value(value);
    Ok(())
}

/// Every root table but `installer_options`, which needs its row's own
/// `platform` / `arch` bound first.
fn substitute_outside_options(root: &mut Table, context: &Context<'_>) -> Result<(), RenderError> {
    for (key, item) in root.iter_mut() {
        let name = key.get().to_string();
        if name == "installer_options" {
            continue;
        }
        substitute::substitute_item(item, &name, context)?;
    }
    Ok(())
}

/// Each `[[installer_options]]` row, with `{platform}` / `{arch}` bound to
/// that row's own declared ids, and `template_optional` applied.
fn render_options(
    root: &mut Table,
    context: &Context<'_>,
) -> Result<Vec<DroppedOption>, RenderError> {
    let rows = match root.get_mut("installer_options") {
        Some(Item::ArrayOfTables(rows)) => rows,
        None => return Ok(Vec::new()),
        // An inline-array spelling would bind no `{platform}` / `{arch}` and
        // would skip the whole row-wise pass, so it is refused rather than
        // rendered half-heartedly.
        Some(_) => {
            return Err(RenderError::template(
                "[[installer_options]] must be written as an array of tables in a template — the \
                 inline-array spelling has no row for `{platform}` / `{arch}` to bind to"
                    .to_string(),
            ));
        }
    };
    let declared = rows.len();
    let mut dropped = Vec::new();
    let mut remove = Vec::new();
    for (index, row) in rows.iter_mut().enumerate() {
        let id = row
            .get("id")
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_string();
        let platform = row
            .get("platform")
            .and_then(Item::as_str)
            .map(str::to_owned);
        let arch = row.get("arch").and_then(Item::as_str).map(str::to_owned);
        let optional = row
            .get("template_optional")
            .and_then(Item::as_bool)
            .unwrap_or(false);
        let mut candidate = row.clone();
        candidate.remove("template_optional");
        let at = format!("installer_options[{index}]");
        let row_context = Context {
            platform: platform.as_deref(),
            arch: arch.as_deref(),
            ..*context
        };
        match substitute_table(&mut candidate, &at, &row_context) {
            Ok(()) => *row = candidate,
            Err(error) if optional && error.kind == RenderErrorKind::MissingDigest => {
                dropped.push(DroppedOption {
                    id,
                    missing: error.subject.clone().unwrap_or_default(),
                });
                remove.push(index);
            }
            Err(error) => return Err(error),
        }
    }
    for index in remove.into_iter().rev() {
        rows.remove(index);
    }
    if declared > 0 && rows.is_empty() {
        return Err(RenderError::new(
            RenderErrorKind::NoOption,
            "installer_options",
            format!(
                "every [[installer_options]] row was dropped for this version: {}",
                dropped
                    .iter()
                    .map(|row| format!("`{}` needs `{}`", row.id, row.missing))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    Ok(dropped)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[path = "render/tests.rs"]
mod tests;
