//! `[package.template]` — the lock record of a version a template
//! produced, and the alarm it exists to raise (PROP-075
//! `#TEMPLATE-LOCK-RECORD`).
//!
//! The record binds by **hash**, not by revision: `rendered_sha256` is the
//! SHA-256 of the full render's TOML bytes, so a later rendering of the
//! same produced version with different digests is a difference the lock
//! can see. A change of the template revision alone, with the same
//! produced version and the same digests, is deliberately *not* an alarm
//! — `#TEMPLATE-FLAG`'s whole purpose is that a template can be fixed
//! without touching a produced version.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD");

use serde::{Deserialize, Serialize};

use super::{Lockfile, TEMPLATE_SCHEMA_VERSION};
use crate::error::Result;

impl Lockfile {
    /// A `[package.template]` table or a `via_channel` needs schema 10:
    /// an older reader's `deny_unknown_fields` would refuse the row
    /// outright, so a lock claiming an older epoch while carrying one is
    /// corruption, never legacy input (AV-FMT-002).
    pub(crate) fn validate_template_records(&self) -> Result<()> {
        if self.meta.schema_version >= TEMPLATE_SCHEMA_VERSION {
            return Ok(());
        }
        for row in &self.packages {
            if row.template.is_some() || row.via_channel.is_some() {
                return Err(crate::Error::InvalidLockfile {
                    reason: format!(
                        "`{}/{}` carries a template record or `via_channel`, which requires \
                         schema{TEMPLATE_SCHEMA_VERSION}",
                        row.group, row.name
                    ),
                });
            }
        }
        Ok(())
    }
}

/// `[package.template]` on a locked package, and the same table on an
/// application installation record.
///
/// ```
/// use vibe_core::manifest::LockedTemplate;
///
/// let t: LockedTemplate = toml::from_str(
///     "revision = \"0.2.0\"\n\
///      rendered_sha256 = \"ab12\"\n\
///      digest_source_sha256 = \"cd34\"\n\
///      source_url = \"https://nodejs.org/dist/index.json\"\n\
///      digest_url = \"https://nodejs.org/dist/v24.22.2/SHASUMS256.txt\"\n",
/// )?;
/// assert_eq!(t.revision.to_string(), "0.2.0");
/// assert_eq!(t.digest_url.as_deref(), Some("https://nodejs.org/dist/v24.22.2/SHASUMS256.txt"));
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedTemplate {
    /// The template package's own `version` — the revision of the
    /// template, never the produced version, which is the row's own
    /// `version`.
    pub revision: semver::Version,
    /// SHA-256 of the **full** render's TOML bytes. A skeleton render is
    /// never hashed into a lock (PROP-075 `#TEMPLATE-TWO-STAGE`).
    pub rendered_sha256: String,
    /// SHA-256 of the fetched digest document's bytes, so a changed
    /// upstream digest document is visible even before a re-render.
    pub digest_source_sha256: String,
    /// The version-list URL as it was resolved **after** remap. Absent
    /// for a `kind = "list"` source, which has no URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// The digest document's URL as it was resolved **after** remap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest_url: Option<String>,
}

/// What a recomputed render disagreed with the record about — the alarm
/// of PROP-075 `#TEMPLATE-LOCK-RECORD`, refused **before any effect**.
///
/// There is no `--trust-template` flag: the two legitimate causes each
/// have an honest answer, re-locking (`vibe update`, which rewrites the
/// record deliberately) or a concrete publication of that version
/// (`#TEMPLATE-CONCRETE-WINS`).
/// ```
/// use vibe_core::manifest::TemplateRenderChanged;
///
/// let alarm = TemplateRenderChanged {
///     package: "org.vibevm.node/node".to_string(),
///     version: "24.22.2".parse()?,
///     recorded_revision: "0.3.0".parse()?,
///     computed_revision: "0.3.0".parse()?,
///     recorded_sha256: "a".repeat(64),
///     computed_sha256: "b".repeat(64),
///     fields: vec!["sha256 = \"aa\" → \"bb\"".to_string()],
/// };
/// let message = alarm.to_string();
/// assert!(message.starts_with("template_render_changed"));
/// assert!(message.contains("org.vibevm.node/node@24.22.2"));
/// assert!(message.contains("--trust-template"));
/// # Ok::<(), semver::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateRenderChanged {
    /// `<group>/<name>`.
    pub package: String,
    /// The produced version.
    pub version: semver::Version,
    /// The template revision the record names.
    pub recorded_revision: semver::Version,
    /// The template revision that rendered now.
    pub computed_revision: semver::Version,
    /// The `rendered_sha256` the record holds.
    pub recorded_sha256: String,
    /// The `rendered_sha256` of the render just computed.
    pub computed_sha256: String,
    /// The digest-bearing fields that differ, as TOML paths, computed
    /// from the two renders.
    pub fields: Vec<String>,
}

impl std::fmt::Display for TemplateRenderChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "template_render_changed: `{}@{}` renders differently than the lock records \
             (template revision {} then, {} now; recorded {}, computed {}",
            self.package,
            self.version,
            self.recorded_revision,
            self.computed_revision,
            short(&self.recorded_sha256),
            short(&self.computed_sha256),
        )?;
        if self.fields.is_empty() {
            f.write_str(
                "; no digest-bearing field differs, so the difference is elsewhere in the \
                 rendered manifest)",
            )?;
        } else {
            write!(f, "; differing fields: {})", self.fields.join(", "))?;
        }
        f.write_str(
            " (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD; \
             fix: re-lock deliberately with `vibe update`, or publish that version concretely — \
             there is no --trust-template)",
        )
    }
}

fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

/// Whether two renders of the same version agree, and if not, which
/// digest-bearing fields differ.
///
/// The comparison is over the TOML text of the two renders, keyed on the
/// digest-bearing field names of PROP-075 `#TEMPLATE-TWO-STAGE`: a
/// difference in `sha256` or `size` is named by its line, and a
/// difference anywhere else leaves the list empty, which the refusal says
/// in as many words rather than claiming no difference.
///
/// ```
/// use vibe_core::manifest::differing_digest_fields;
///
/// let before = "[[installer_options.steps]]\nid = \"unpack\"\nsha256 = \"aa\"\n";
/// let after = "[[installer_options.steps]]\nid = \"unpack\"\nsha256 = \"bb\"\n";
/// assert_eq!(differing_digest_fields(before, after), vec!["sha256 = \"aa\" → \"bb\""]);
/// assert!(differing_digest_fields(before, before).is_empty());
/// ```
pub fn differing_digest_fields(recorded: &str, computed: &str) -> Vec<String> {
    let digest_lines = |text: &str| -> Vec<(String, String)> {
        text.lines()
            .filter_map(|line| {
                let (key, value) = line.split_once('=')?;
                let key = key.trim();
                (key == "sha256" || key == "size")
                    .then(|| (key.to_string(), value.trim().to_string()))
            })
            .collect()
    };
    let before = digest_lines(recorded);
    let after = digest_lines(computed);
    let mut differing = Vec::new();
    for index in 0..before.len().max(after.len()) {
        match (before.get(index), after.get(index)) {
            (Some(a), Some(b)) if a != b => {
                differing.push(format!("{} = {} → {}", a.0, a.1, b.1));
            }
            (Some(a), None) => differing.push(format!("{} = {} → absent", a.0, a.1)),
            (None, Some(b)) => differing.push(format!("{} absent → {}", b.0, b.1)),
            _ => {}
        }
    }
    differing
}
