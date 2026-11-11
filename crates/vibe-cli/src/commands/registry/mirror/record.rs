//! The mirror record a target keeps per source registry,
//! `_index/mirrors/<source>.toml` (PROP-002 `#registry-mirror`): enough to
//! mirror back — the source registry's identity and naming, and per
//! version the repository, the tag exactly as it was (object kind, tagger,
//! message), the commit, its tree, author, committer and message.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::git_source::TagFacts;

pub(super) const SCHEMA: u32 = 1;

/// `_index/mirrors/<source>.toml`, relative to the registry root.
pub(super) fn record_path(source: &str) -> String {
    format!(
        "{}/mirrors/{source}.toml",
        vibe_core::layout_registry::INDEX_DIR
    )
}

/// `_index/mirrors/bundles/<source>/<repo>.bundle`.
pub(super) fn bundle_path(source: &str, repo: &str) -> String {
    format!(
        "{}/mirrors/bundles/{source}/{repo}.bundle",
        vibe_core::layout_registry::INDEX_DIR
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MirrorRecord {
    pub schema: u32,
    pub source: SourceRecord,
    pub sync: SyncRecord,
    /// The selection the last run that had one used, so a later bare
    /// `vibe registry mirror <source> --into <target>` repeats it and adds
    /// only what newly matches (PROP-075 `#MIRROR-CONFIG`). Additive: the
    /// record's `schema` stays 1 because a reader that ignores this table
    /// and the `[[template]]` rows beside it behaves exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<SelectRecord>,
    #[serde(default, rename = "version")]
    pub versions: Vec<VersionRecord>,
    /// The alias rows this mirror last CARRIED from the source
    /// (PROP-008 §2.6). Not the target's table — that is
    /// `_index/aliases.toml`, which the mirror owner may edit — but the
    /// values the source had when they were copied, which is the only
    /// way a later run can tell a local hand from an untouched copy.
    /// Additive: a reader that ignores these rows behaves exactly as
    /// before, so the record's `schema` stays 1.
    #[serde(default, rename = "alias", skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<AliasRecord>,
}

/// One alias row as it stood in the source when this mirror copied it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct AliasRecord {
    pub alias: String,
    /// The `<group>/<name>` the SOURCE gave, verbatim.
    pub target: String,
}

/// `[select]` and the `[[template]]` rows beside it, as recorded.
///
/// The spellings are the authored ones, not a lowered form: a recorded
/// `versions = [">=22"]` is read back as the requirement it was, and a
/// channel stays a channel so a re-run lowers it against the version list
/// of the day rather than the one it was written on.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SelectRecord {
    #[serde(default)]
    pub all: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    /// Serialised as `[[select.template]]`, so the rows sit inside the
    /// table they belong to and a `[[version]]` row after them is still
    /// read as the record's own.
    #[serde(default, rename = "template", skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<TemplateRecord>,
}

/// One recorded `[[template]]` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TemplateRecord {
    /// `<group>/<name>`.
    pub package: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newest: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub archs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installers: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub payloads: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SourceRecord {
    pub name: String,
    pub url: String,
    /// `github` | `gitverse` | `git` | `monorepo` | `localdir`.
    #[serde(rename = "type")]
    pub kind: String,
    /// How `<group>/<name>` maps to a repository name (`fqdn`, `kind-name`,
    /// `name`, `kind/name`).
    pub naming: String,
    #[serde(rename = "ref")]
    pub refname: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SyncRecord {
    /// The vibe that wrote the record last.
    pub vibe: String,
    pub first: String,
    pub last: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct VersionRecord {
    /// `<group>/<name>`.
    pub package: String,
    pub version: String,
    /// The source repository (canonical url) and its name under `naming`.
    pub repository: String,
    pub repo: String,
    /// The version directory's Git tree equals `facts.tree` (checked when
    /// it was written).
    pub tree_verified: bool,
    #[serde(flatten)]
    pub facts: TagFacts,
}

impl MirrorRecord {
    /// Read the record from a registry tree; `None` when absent.
    pub(super) fn read(root: &Path, source: &str) -> Result<Option<Self>> {
        let path = root.join(record_path(source));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error).with_context(|| format!("reading `{}`", path.display()));
            }
        };
        let record: MirrorRecord =
            toml::from_str(&text).with_context(|| format!("parsing `{}`", path.display()))?;
        if record.schema != SCHEMA {
            bail!(
                "`{}` has schema {}, this vibe reads {SCHEMA} \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
                 fix: mirror with the vibe that wrote it, or a newer one)",
                path.display(),
                record.schema
            );
        }
        Ok(Some(record))
    }

    /// Replace (or add) the rows of `versions`, keeping the others; rows
    /// sorted by package then version.
    pub(super) fn merge(&mut self, versions: Vec<VersionRecord>) {
        for row in versions {
            self.versions
                .retain(|old| !(old.package == row.package && old.version == row.version));
            self.versions.push(row);
        }
        self.versions.sort_by(|a, b| {
            let version = |v: &VersionRecord| semver::Version::parse(&v.version).ok();
            (a.package.as_str(), version(a)).cmp(&(b.package.as_str(), version(b)))
        });
    }

    /// Replace the recorded alias rows with what this run carried.
    /// Wholesale rather than row-by-row: the rows ARE the statement
    /// "this is what the source said last time we looked", and a stale
    /// row left behind would make the next run read an untouched copy
    /// as a local edit.
    pub(super) fn record_aliases(&mut self, carried: Vec<(String, String)>) {
        self.aliases = carried
            .into_iter()
            .map(|(alias, target)| AliasRecord { alias, target })
            .collect();
        self.aliases.sort_by(|a, b| a.alias.cmp(&b.alias));
    }

    /// The alias rows the previous run recorded carrying.
    pub(super) fn carried_aliases(&self) -> Vec<(String, String)> {
        self.aliases
            .iter()
            .map(|row| (row.alias.clone(), row.target.clone()))
            .collect()
    }

    pub(super) fn render(&self) -> Result<String> {
        Ok(format!(
            "# Written by `vibe registry mirror` (PROP-002 #registry-mirror): what this\n\
             # registry copied from `{}`, kept so it can be mirrored back.\n{}",
            self.source.name,
            toml::to_string(self).context("serialising the mirror record")?
        ))
    }
}
