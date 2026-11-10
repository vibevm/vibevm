//! `mirror.toml` — the declarative selection a mirror run reads with
//! `--config` (PROP-075 `#MIRROR-CONFIG-SCHEMA`).
//!
//! This module is the **document**: its typed shape, its refusals and
//! nothing else. What the document means for a run — whose packages are
//! admitted, which template rows apply, where payloads go — is
//! [`super::selection::Selection`], so the reader can be tested without a
//! registry and the selection without a file.
//!
//! Every table is `deny_unknown_fields`, as every authored document in the
//! tree is: a misspelt key is a refusal naming it, never a silently
//! ignored declaration.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The only schema this vibe reads; any other value is refused outright,
/// exactly as `_index/mirrors/<source>.toml` is (`super::record::SCHEMA`).
pub(super) const SCHEMA: u32 = 1;

const SPEC: &str = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA";

/// `mirror.toml` as written.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConfigDocument {
    pub schema: u32,
    #[serde(default)]
    pub select: Option<SelectDocument>,
    #[serde(default, rename = "template")]
    pub templates: Vec<TemplateDocument>,
    #[serde(default)]
    pub payloads: Option<PayloadsDocument>,
}

/// `[select]` — ordinary packages, on the two axes today's `--package` /
/// `--group` already have, plus a subtraction.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectDocument {
    #[serde(default)]
    pub all: bool,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    /// Either spelling: a bare group id, or `<group>/<name>`.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// One `[[template]]` row — a template package and which of the versions
/// it produces this mirror wants.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TemplateDocument {
    /// `<group>/<name>`.
    pub package: String,
    /// Version requirements, ORed.
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    /// For every major version present after the other filters, keep the
    /// newest N.
    #[serde(default)]
    pub newest: Option<u32>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub archs: Vec<String>,
    /// Installer-option ids, which win over `platforms` / `archs`.
    #[serde(default)]
    pub installers: Vec<String>,
    #[serde(default)]
    pub payloads: bool,
}

/// `[payloads]` — the budget and the drop payload projection writes into.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PayloadsDocument {
    /// `"40 GiB"` or an integer byte count.
    #[serde(default)]
    pub max_total: Option<toml::Value>,
    #[serde(default)]
    pub into: Option<PathBuf>,
}

/// Read and validate `mirror.toml`.
///
/// Validation here is everything decidable from the document alone: the
/// schema, the spellings of every id, requirement and channel name, and
/// the byte budget. What needs the run's context — an empty selection, a
/// drop that is the registry target, a template row against a source that
/// cannot carry one — is refused by [`super::selection`].
pub(super) fn read(path: &Path) -> Result<Validated> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => bail!(
            "`--config {}` names a file that does not exist \
             (violates {SPEC}; fix: write the mirror configuration there, or drop `--config` and \
             select with `--package` / `--group`)",
            path.display()
        ),
        Err(error) => {
            return Err(error).with_context(|| format!("reading `{}`", path.display()));
        }
    };
    let document: ConfigDocument =
        toml::from_str(&text).with_context(|| format!("parsing `{}`", path.display()))?;
    if document.schema != SCHEMA {
        bail!(
            "`{}` has schema {}, this vibe reads {SCHEMA} \
             (violates {SPEC}; fix: mirror with the vibe that wrote it, or a newer one)",
            path.display(),
            document.schema
        );
    }
    validate(&document, path)
}

/// A document whose every spelling parsed, with the parsed values kept so
/// nothing is read twice.
#[derive(Debug, Clone)]
pub(super) struct Validated {
    pub path: PathBuf,
    pub select: SelectDocument,
    pub templates: Vec<ValidatedTemplate>,
    pub payloads: ValidatedPayloads,
}

/// One `[[template]]` row with its coordinate and requirements parsed.
#[derive(Debug, Clone)]
pub(super) struct ValidatedTemplate {
    pub group: String,
    pub name: String,
    pub versions: Vec<semver::VersionReq>,
    pub channels: Vec<vibe_core::package_ref::ChannelName>,
    pub newest: Option<u32>,
    pub platforms: Vec<String>,
    pub archs: Vec<String>,
    pub installers: Vec<String>,
    pub payloads: bool,
}

impl ValidatedTemplate {
    pub fn package(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct ValidatedPayloads {
    pub max_total: Option<u64>,
    pub into: Option<PathBuf>,
}

fn validate(document: &ConfigDocument, path: &Path) -> Result<Validated> {
    let shown = path.display().to_string();
    let select = document.select.clone().unwrap_or_default();
    for group in &select.groups {
        vibe_core::Group::parse(group)
            .with_context(|| format!("`[select] groups` of `{shown}`"))?;
    }
    for package in &select.packages {
        split_package(package, "[select] packages", &shown)?;
    }
    for entry in &select.exclude {
        if entry.contains('/') {
            split_package(entry, "[select] exclude", &shown)?;
        } else {
            vibe_core::Group::parse(entry)
                .with_context(|| format!("`[select] exclude` of `{shown}`"))?;
        }
    }
    let mut templates = Vec::with_capacity(document.templates.len());
    for row in &document.templates {
        templates.push(validate_template(row, &shown)?);
    }
    for (index, row) in templates.iter().enumerate() {
        if templates[..index]
            .iter()
            .any(|seen| seen.package() == row.package())
        {
            bail!(
                "`{shown}` declares `{}` in two `[[template]]` rows \
                 (violates {SPEC}; fix: one row per template package — merge the two)",
                row.package()
            );
        }
    }
    Ok(Validated {
        path: path.to_path_buf(),
        select,
        templates,
        payloads: validate_payloads(document.payloads.as_ref(), &shown)?,
    })
}

fn validate_template(row: &TemplateDocument, shown: &str) -> Result<ValidatedTemplate> {
    let (group, name) = split_package(&row.package, "[[template]] package", shown)?;
    let mut versions = Vec::with_capacity(row.versions.len());
    for spelling in &row.versions {
        versions.push(semver::VersionReq::parse(spelling).map_err(|error| {
            anyhow::anyhow!(
                "`[[template]] versions` of `{shown}` carries `{spelling}`, which is not a \
                 version requirement: {error} \
                 (violates {SPEC}; fix: write a semver requirement such as `>=22` or `^24.1`)"
            )
        })?);
    }
    let mut channels = Vec::with_capacity(row.channels.len());
    for spelling in &row.channels {
        channels.push(
            vibe_core::package_ref::ChannelName::parse(spelling).ok_or_else(|| {
                anyhow::anyhow!(
                    "`[[template]] channels` of `{shown}` carries `{spelling}`, which is not a \
                     channel name \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX; \
                     fix: a channel is `[a-z][a-z0-9.-]*`, optionally one `/` and a second such \
                     segment — `lts`, `lts/jod`)"
                )
            })?,
        );
    }
    if row.newest == Some(0) {
        bail!(
            "`[[template]] newest = 0` of `{shown}` ({}) would keep no version of any major \
             (violates {SPEC}; fix: `newest` keeps the newest N per major — write 1 or more, or \
             drop the key)",
            row.package
        );
    }
    for (key, ids) in [
        ("platforms", &row.platforms),
        ("archs", &row.archs),
        ("installers", &row.installers),
    ] {
        for id in ids {
            if id.trim().is_empty() {
                bail!(
                    "`[[template]] {key}` of `{shown}` ({}) carries an empty id \
                     (violates {SPEC}; fix: name the ids the template's \
                     `[[installer_options]]` rows declare, or drop the key)",
                    row.package
                );
            }
        }
    }
    Ok(ValidatedTemplate {
        group,
        name,
        versions,
        channels,
        newest: row.newest,
        platforms: row.platforms.clone(),
        archs: row.archs.clone(),
        installers: row.installers.clone(),
        payloads: row.payloads,
    })
}

fn validate_payloads(
    payloads: Option<&PayloadsDocument>,
    shown: &str,
) -> Result<ValidatedPayloads> {
    let Some(payloads) = payloads else {
        return Ok(ValidatedPayloads::default());
    };
    let max_total = match &payloads.max_total {
        None => None,
        Some(toml::Value::Integer(bytes)) if *bytes >= 0 => Some(*bytes as u64),
        Some(toml::Value::String(text)) => Some(parse_bytes(text).map_err(|reason| {
            anyhow::anyhow!(
                "`[payloads] max_total` of `{shown}` is `{text}`: {reason} \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET; \
                 fix: write a byte budget such as `\"40 GiB\"` or an integer count)"
            )
        })?),
        Some(other) => bail!(
            "`[payloads] max_total` of `{shown}` is {}, not a byte budget \
             (violates spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET; \
             fix: write `\"40 GiB\"` or a non-negative integer count)",
            other.type_str()
        ),
    };
    if max_total == Some(0) {
        bail!(
            "`[payloads] max_total = 0` of `{shown}` admits no payload at all \
             (violates spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET; \
             fix: write the budget you mean, or drop `payloads = true` from the \
             `[[template]]` rows)"
        );
    }
    Ok(ValidatedPayloads {
        max_total,
        into: payloads.into.clone(),
    })
}

/// `<group>/<name>`, with both halves validated.
fn split_package(spelling: &str, key: &str, shown: &str) -> Result<(String, String)> {
    let (group, name) = spelling.split_once('/').ok_or_else(|| {
        anyhow::anyhow!(
            "`{key}` of `{shown}` carries `{spelling}`, which is not `<group>/<name>` \
             (violates {SPEC}; fix: name the package with its group, e.g. \
             `org.vibevm.node/node`)"
        )
    })?;
    vibe_core::Group::parse(group).with_context(|| format!("`{key}` of `{shown}`"))?;
    if name.is_empty() || name.contains('/') {
        bail!(
            "`{key}` of `{shown}` carries `{spelling}`, whose name part is not a package name \
             (violates {SPEC}; fix: write exactly one `/`, e.g. `org.vibevm.node/node`)"
        );
    }
    Ok((group.to_string(), name.to_string()))
}

/// A byte count: a bare integer, or a figure with a unit. Decimal units
/// are powers of 1000 and `KiB` / `MiB` / `GiB` / `TiB` of 1024 — the same
/// vocabulary `vibe_core::progress::transfer::TransferLimit::parse` reads
/// for a rate, so the two spellings of a size in this tree agree.
pub(super) fn parse_bytes(text: &str) -> Result<u64, String> {
    let figure = text.trim();
    let split = figure
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(figure.len());
    let (number, unit) = figure.split_at(split);
    let value: f64 = number
        .parse()
        .map_err(|_| "it does not start with a number".to_string())?;
    if value < 0.0 {
        return Err("a budget cannot be negative".to_string());
    }
    let scale: f64 = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" => 1e3,
        "m" | "mb" => 1e6,
        "g" | "gb" => 1e9,
        "t" | "tb" => 1e12,
        "kib" => 1024.0,
        "mib" => 1024.0 * 1024.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        other => {
            return Err(format!(
                "`{other}` is an unknown unit; use B, KB, MB, GB, TB or KiB, MiB, GiB, TiB"
            ));
        }
    };
    Ok((value * scale).round() as u64)
}

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;
