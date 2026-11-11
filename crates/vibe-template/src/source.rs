//! The pure inputs of a render: a version record, a digest table, and the
//! `[template]` declaration read out of the template document itself.
//!
//! Nothing here knows where the bytes came from — a transport, a cache, a
//! mirror snapshot or a test fixture all hand it the same strings
//! (PROP-075 `#TEMPLATE-RENDER-LOCATION`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-SOURCE-KINDS");

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use vibe_core::manifest::{DigestSource, Scalar, TemplateSection, VersionSource};

use crate::error::{RenderError, RenderErrorKind};

/// One produced version and the fields a `{meta.PATH}` variable may read.
///
/// ```
/// use vibe_core::manifest::Scalar;
/// use vibe_template::VersionRecord;
///
/// let mut record = VersionRecord::plain("24.22.2")?;
/// record.raw = "v24.22.2".to_string();
/// record.meta.insert("lts".to_string(), Scalar::String("Krypton".into()));
/// assert_eq!(record.version.minor, 22);
/// assert_eq!(record.meta["lts"].to_string(), "Krypton");
/// # Ok::<(), vibe_template::RenderError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRecord {
    /// The produced version, canonical semver — what `{version}` renders.
    pub version: semver::Version,
    /// The source's own spelling of it — what `{version_raw}` renders
    /// (`v24.22.2` for Node).
    pub raw: String,
    /// Dotted paths into the source's record that resolve to a scalar.
    pub meta: BTreeMap<String, Scalar>,
    /// Dotted paths that exist but hold an array or a table, so a
    /// `{meta.…}` naming one is refused *as such* rather than as absent.
    composite: BTreeSet<String>,
}

impl VersionRecord {
    /// A record with no metadata, for a `kind = "list"` row or a test.
    ///
    /// ```
    /// use vibe_template::VersionRecord;
    ///
    /// let record = VersionRecord::plain("24.22.2")?;
    /// assert_eq!(record.raw, "24.22.2");
    /// assert_eq!(record.version.major, 24);
    /// # Ok::<(), vibe_template::RenderError>(())
    /// ```
    pub fn plain(version: &str) -> Result<Self, RenderError> {
        Self::with_meta(version, version, BTreeMap::new(), BTreeSet::new())
    }

    fn with_meta(
        version: &str,
        raw: &str,
        meta: BTreeMap<String, Scalar>,
        composite: BTreeSet<String>,
    ) -> Result<Self, RenderError> {
        let version = version.parse::<semver::Version>().map_err(|error| {
            RenderError::new(
                RenderErrorKind::Template,
                String::new(),
                format!("`{version}` is not a semantic version: {error}"),
            )
        })?;
        Ok(Self {
            version,
            raw: raw.to_string(),
            meta,
            composite,
        })
    }

    /// Whether this dotted path exists but holds an array or a table.
    pub(crate) fn is_composite(&self, path: &str) -> bool {
        self.composite.contains(path)
    }

    /// Every record of one JSON document, newest-first as the document
    /// orders them, with the reason each skipped record was skipped.
    ///
    /// A record whose version string does not parse as semver after
    /// `strip_prefix` is **skipped with a reported reason**, never an
    /// error: an upstream index legitimately carries shapes vibe does not
    /// understand (PROP-075 `#TEMPLATE-SOURCE-KINDS`).
    fn from_json_document(
        document: &str,
        select: &str,
        version_field: &str,
        strip_prefix: Option<&str>,
    ) -> Result<(Vec<Self>, Vec<String>), RenderError> {
        let value: serde_json::Value = serde_json::from_str(document).map_err(|error| {
            RenderError::template(format!("the version document is not JSON: {error}"))
        })?;
        let pointed = if select.is_empty() {
            Some(&value)
        } else {
            value.pointer(select)
        };
        let array = pointed
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                RenderError::template(format!(
                    "[template.versions].select `{select}` does not point at an array of objects \
                     — found {}",
                    pointed.map_or("nothing".to_string(), describe)
                ))
            })?;
        let mut records = Vec::new();
        let mut skipped = Vec::new();
        for element in array {
            let Some(object) = element.as_object() else {
                skipped.push(format!(
                    "a record that is {}, not an object",
                    describe(element)
                ));
                continue;
            };
            let Some(spelling) = object.get(version_field).and_then(|v| v.as_str()) else {
                skipped.push(format!(
                    "a record with no string `{version_field}`: {}",
                    one_line(element)
                ));
                continue;
            };
            let bare = strip_prefix.map_or(spelling, |prefix| {
                spelling.strip_prefix(prefix).unwrap_or(spelling)
            });
            let mut meta = BTreeMap::new();
            let mut composite = BTreeSet::new();
            flatten(element, String::new(), &mut meta, &mut composite);
            match Self::with_meta(bare, spelling, meta, composite) {
                Ok(record) => records.push(record),
                Err(error) => skipped.push(format!("`{spelling}`: {}", error.message)),
            }
        }
        Ok((records, skipped))
    }
}

/// Every version a template produces, from its declaration plus — for a
/// `kind = "json"` source — the fetched document.
///
/// The `min` / `max` / `prerelease` filters are applied here, before
/// anything else sees a version, exactly as PROP-075
/// `#TEMPLATE-SOURCE-KINDS` prescribes. A source that yields no usable
/// version at all is an error; individual unparseable records are skipped
/// with a reason.
///
/// ```
/// use vibe_core::manifest::TemplateSection;
/// use vibe_template::version_records;
///
/// let section: TemplateSection = toml::from_str(
///     "[versions]\nkind = \"json\"\nurl = \"https://n.invalid/index.json\"\n\
///      select = \"\"\nstrip_prefix = \"v\"\nmin = \"20.0.0\"\n\
///      [digests]\nkind = \"sha256sums\"\nurl = \"https://n.invalid/{version}\"\n",
/// ).unwrap();
/// let (records, skipped) = version_records(
///     &section,
///     Some(r#"[{"version":"v24.1.0"},{"version":"v18.0.0"},{"version":"nope"}]"#),
/// )?;
/// assert_eq!(records.len(), 1, "18 is below min, `nope` is skipped");
/// assert_eq!(records[0].raw, "v24.1.0");
/// assert_eq!(skipped.len(), 1);
/// # Ok::<(), vibe_template::RenderError>(())
/// ```
pub fn version_records(
    section: &TemplateSection,
    document: Option<&str>,
) -> Result<(Vec<VersionRecord>, Vec<String>), RenderError> {
    let (mut records, mut skipped) = match &section.versions {
        VersionSource::Json {
            select,
            version_field,
            strip_prefix,
            ..
        } => {
            let document = document.ok_or_else(|| {
                RenderError::template(
                    "a `kind = \"json\"` version source needs its fetched document".to_string(),
                )
            })?;
            VersionRecord::from_json_document(
                document,
                select,
                version_field,
                strip_prefix.as_deref(),
            )?
        }
        VersionSource::List { items } => {
            let mut records = Vec::new();
            for item in items {
                records.push(VersionRecord::with_meta(
                    &item.version,
                    &item.version,
                    item.meta.clone(),
                    BTreeSet::new(),
                )?);
            }
            (records, Vec::new())
        }
    };
    if let VersionSource::Json {
        min,
        max,
        prerelease,
        ..
    } = &section.versions
    {
        records.retain(|record| {
            min.as_ref().is_none_or(|min| &record.version >= min)
                && max.as_ref().is_none_or(|max| &record.version <= max)
                && (prerelease.includes_prerelease() || record.version.pre.is_empty())
        });
    }
    if records.is_empty() {
        skipped.truncate(8);
        return Err(RenderError::template(format!(
            "the version source yielded no usable version{}",
            if skipped.is_empty() {
                String::new()
            } else {
                format!(" — skipped: {}", skipped.join("; "))
            }
        )));
    }
    Ok((records, skipped))
}

/// The `[template]` table of a template document.
///
/// The document is read as TOML, **not** as a typed `Manifest`: a template
/// whose numeric field varies holds a string where the manifest expects a
/// number, and the declaration must still be readable (PROP-075
/// `#TEMPLATE-DOCUMENT-READING`).
///
/// ```
/// use vibe_template::template_section;
///
/// let section = template_section(
///     "[package]\nname = \"a\"\ntemplate = true\n\
///      [template.versions]\nkind = \"list\"\n\
///      [[template.versions.item]]\nversion = \"1.0.0\"\n\
///      [template.digests]\nkind = \"sha256sums\"\nurl = \"https://e.invalid/{version}\"\n\
///      [[installer_options.steps]]\nsize = \"{size:x}\"\n",
/// )?;
/// assert!(!section.has_sizes());
/// # Ok::<(), vibe_template::RenderError>(())
/// ```
pub fn template_section(template_toml: &str) -> Result<TemplateSection, RenderError> {
    #[derive(Deserialize)]
    struct OnlyTemplate {
        template: Option<TemplateSection>,
    }

    let document: OnlyTemplate = toml::from_str(template_toml).map_err(|error| {
        RenderError::template(format!(
            "the template's [template] table is unreadable: {error}"
        ))
    })?;
    document.template.ok_or_else(|| {
        RenderError::template(
            "the template manifest carries no [template] table — a template must say where its \
             versions and digests come from"
                .to_string(),
        )
    })
}

/// One file's digest and, when the source carries it, its byte size.
///
/// ```
/// use vibe_template::DigestEntry;
///
/// let entry = DigestEntry { sha256: "a".repeat(64), size: Some(58_299_872) };
/// assert_eq!(entry.sha256.len(), 64);
/// assert_eq!(entry.size, Some(58_299_872));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestEntry {
    pub sha256: String,
    pub size: Option<u64>,
}

/// The digests of one produced version, by file name.
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_template::{DigestEntry, DigestTable};
///
/// let mut entries = BTreeMap::new();
/// entries.insert(
///     "node-v24.22.2-linux-x64.tar.gz".to_string(),
///     DigestEntry { sha256: "a".repeat(64), size: None },
/// );
/// let table = DigestTable::from_entries(entries);
/// assert!(table.get("node-v24.22.2-linux-x64.tar.gz").is_some());
/// assert!(DigestTable::default().get("anything").is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DigestTable {
    entries: BTreeMap<String, DigestEntry>,
}

impl DigestTable {
    /// `sha256sum(1)` output: one `<digest><space>[*]<name>` pair per line,
    /// other lines skipped, a duplicate name an error.
    ///
    /// ```
    /// use vibe_template::DigestTable;
    ///
    /// let table = DigestTable::parse_sha256sums(&format!(
    ///     "not a pair\n{}  *node-v1.tar.gz\n", "b".repeat(64)
    /// ))?;
    /// assert_eq!(table.get("node-v1.tar.gz").map(|e| e.sha256.as_str()), Some("b".repeat(64).as_str()));
    /// assert!(table.get("absent").is_none());
    /// # Ok::<(), vibe_template::RenderError>(())
    /// ```
    pub fn parse_sha256sums(text: &str) -> Result<Self, RenderError> {
        let mut entries: BTreeMap<String, DigestEntry> = BTreeMap::new();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let (Some(digest), Some(name), None) = (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                continue;
            }
            let name = name.strip_prefix('*').unwrap_or(name);
            if entries
                .insert(
                    name.to_string(),
                    DigestEntry {
                        sha256: digest.to_ascii_lowercase(),
                        size: None,
                    },
                )
                .is_some()
            {
                return Err(RenderError::template(format!(
                    "the digest document names `{name}` twice"
                )));
            }
        }
        Ok(Self { entries })
    }

    /// A JSON checksum manifest, read through the declared source.
    pub fn parse(section: &TemplateSection, text: &str) -> Result<Self, RenderError> {
        match &section.digests {
            // A template that declares no digest source has no digest
            // document to read and no digest variable to fill — the
            // empty table is the whole truth about its inputs
            // (PROP-075 `#TEMPLATE-NO-DIGEST`).
            None => Ok(Self::default()),
            Some(DigestSource::Sha256Sums { .. }) => Self::parse_sha256sums(text),
            Some(DigestSource::Json {
                select,
                file_field,
                digest_field,
                size_field,
                ..
            }) => Self::parse_json(
                text,
                select,
                file_field,
                digest_field,
                size_field.as_deref(),
            ),
        }
    }

    fn parse_json(
        text: &str,
        select: &str,
        file_field: &str,
        digest_field: &str,
        size_field: Option<&str>,
    ) -> Result<Self, RenderError> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|error| {
            RenderError::template(format!("the digest document is not JSON: {error}"))
        })?;
        let pointed = if select.is_empty() {
            Some(&value)
        } else {
            value.pointer(select)
        };
        let array = pointed
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                RenderError::template(format!(
                    "[template.digests].select `{select}` does not point at an array of objects"
                ))
            })?;
        let mut entries: BTreeMap<String, DigestEntry> = BTreeMap::new();
        for element in array {
            let Some(object) = element.as_object() else {
                continue;
            };
            let (Some(name), Some(digest)) = (
                object.get(file_field).and_then(|v| v.as_str()),
                object.get(digest_field).and_then(|v| v.as_str()),
            ) else {
                continue;
            };
            let size = size_field
                .and_then(|field| object.get(field))
                .and_then(size_of);
            if entries
                .insert(
                    name.to_string(),
                    DigestEntry {
                        sha256: digest.to_ascii_lowercase(),
                        size,
                    },
                )
                .is_some()
            {
                return Err(RenderError::template(format!(
                    "the digest document names `{name}` twice"
                )));
            }
        }
        Ok(Self { entries })
    }

    /// The entry for one file name, if the document names it.
    pub fn get(&self, file: &str) -> Option<&DigestEntry> {
        self.entries.get(file)
    }

    /// Build a table directly, for a caller that already has the pairs.
    pub fn from_entries(entries: BTreeMap<String, DigestEntry>) -> Self {
        Self { entries }
    }
}

fn size_of(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

/// Every dotted path of a JSON value that resolves to a scalar, with the
/// composite paths recorded apart so a refusal can say which it was.
fn flatten(
    value: &serde_json::Value,
    prefix: String,
    meta: &mut BTreeMap<String, Scalar>,
    composite: &mut BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            if !prefix.is_empty() {
                composite.insert(prefix.clone());
            }
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(child, path, meta, composite);
            }
        }
        serde_json::Value::Array(_) | serde_json::Value::Null => {
            if !prefix.is_empty() {
                composite.insert(prefix);
            }
        }
        serde_json::Value::Bool(flag) => {
            meta.insert(prefix, Scalar::Bool(*flag));
        }
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(integer) => {
                meta.insert(prefix, Scalar::Integer(integer));
            }
            // A non-integral number stays out of `meta`, so a variable
            // naming it is refused as composite rather than rendered with a
            // platform-dependent spelling.
            None => {
                composite.insert(prefix);
            }
        },
        serde_json::Value::String(text) => {
            meta.insert(prefix, Scalar::String(text.clone()));
        }
    }
}

fn describe(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "a boolean".to_string(),
        serde_json::Value::Number(_) => "a number".to_string(),
        serde_json::Value::String(_) => "a string".to_string(),
        serde_json::Value::Array(_) => "an array".to_string(),
        serde_json::Value::Object(_) => "an object".to_string(),
    }
}

fn one_line(value: &serde_json::Value) -> String {
    let mut text = value.to_string();
    text.truncate(120);
    text
}

#[cfg(test)]
#[path = "source/tests.rs"]
mod tests;
