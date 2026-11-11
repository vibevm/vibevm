//! The refusals of `[template]`, each naming its TOML path (PROP-059
//! `#declaration`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-SOURCE-KINDS");

use crate::manifest::package::application::credential_free_https;
use crate::manifest::{FieldError, FieldPath};

use super::{ChannelRule, DigestSource, TemplateSection, VersionSource};

/// The channel-name grammar, as one sentence a refusal can quote.
pub const CHANNEL_NAME_RULE: &str = "a channel name is a lowercase letter followed by lowercase letters, digits, `.` or `-`, \
     optionally one `/` and a second such segment (`lts`, `lts/jod`, `nightly`)";

/// Whether `name` is a channel name of PROP-075 `#CHANNEL-SYNTAX`.
///
/// ```
/// use vibe_core::manifest::channel_name_is_valid;
///
/// assert!(channel_name_is_valid("lts"));
/// assert!(channel_name_is_valid("lts/jod"));
/// assert!(!channel_name_is_valid("LTS"));
/// assert!(!channel_name_is_valid("lts/jod/extra"));
/// assert!(!channel_name_is_valid("22"));
/// ```
pub fn channel_name_is_valid(name: &str) -> bool {
    let mut parts = name.split('/');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    parts.next().is_none() && segment_is_valid(first) && second.is_none_or(segment_is_valid)
}

fn segment_is_valid(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
        })
}

impl TemplateSection {
    /// Every rule of the declaration that needs neither the network nor a
    /// fetched document. `at` is the table's address, `template`.
    pub fn validate(&self, at: &FieldPath) -> Result<(), FieldError> {
        self.validate_versions(&at.table("versions"))?;
        self.validate_digests(&at.table("digests"))?;
        for (index, channel) in self.channels.iter().enumerate() {
            validate_channel(channel, &at.table("channel").index(index))?;
        }
        Ok(())
    }

    fn validate_versions(&self, at: &FieldPath) -> Result<(), FieldError> {
        match &self.versions {
            VersionSource::Json {
                url,
                select,
                version_field,
                min,
                max,
                ..
            } => {
                if !credential_free_https(url) {
                    return Err(FieldError::new(
                        at.field("url"),
                        "[template.versions].url must be a credential-free HTTPS URL".to_string(),
                    ));
                }
                if url.contains(['{', '}']) {
                    return Err(FieldError::new(
                        at.field("url"),
                        "[template.versions].url carries no template variable — the version list \
                         cannot depend on a version"
                            .to_string(),
                    ));
                }
                validate_pointer(select, &at.field("select"), "[template.versions].select")?;
                if version_field.trim().is_empty() {
                    return Err(FieldError::new(
                        at.field("version_field"),
                        "[template.versions].version_field must name a key".to_string(),
                    ));
                }
                if let (Some(min), Some(max)) = (min, max)
                    && min > max
                {
                    return Err(FieldError::new(
                        at.field("min"),
                        format!("[template.versions].min `{min}` is above `max` `{max}`"),
                    ));
                }
                Ok(())
            }
            VersionSource::List { items } => {
                if items.is_empty() {
                    return Err(FieldError::new(
                        at.table("item"),
                        "[template.versions] of kind `list` must carry at least one \
                         [[template.versions.item]]"
                            .to_string(),
                    ));
                }
                let mut seen = std::collections::BTreeSet::new();
                for (index, item) in items.iter().enumerate() {
                    let row = at.table("item").index(index);
                    let version = item.version.parse::<semver::Version>().map_err(|error| {
                        FieldError::new(
                            row.field("version"),
                            format!(
                                "[[template.versions.item]].version `{}` is not a semantic \
                                 version: {error}",
                                item.version
                            ),
                        )
                    })?;
                    if !seen.insert(version.clone()) {
                        return Err(FieldError::new(
                            row.field("version"),
                            format!("[[template.versions.item]].version `{version}` is duplicated"),
                        ));
                    }
                }
                Ok(())
            }
        }
    }

    fn validate_digests(&self, at: &FieldPath) -> Result<(), FieldError> {
        // A template that declares no digest source is legal and has
        // nothing to validate here; whether it is *allowed* to declare
        // none is decided where the variables are read, because that is
        // where "no downloaded input" can actually be checked
        // (PROP-075 `#TEMPLATE-NO-DIGEST`).
        let Some(digests) = &self.digests else {
            return Ok(());
        };
        let (url, declared) = match digests {
            DigestSource::Sha256Sums { url, sizes } => (url, *sizes),
            DigestSource::Json {
                url,
                select,
                file_field,
                digest_field,
                sizes,
                ..
            } => {
                validate_pointer(select, &at.field("select"), "[template.digests].select")?;
                for (field, value) in [("file_field", file_field), ("digest_field", digest_field)] {
                    if value.trim().is_empty() {
                        return Err(FieldError::new(
                            at.field(field),
                            format!("[template.digests].{field} must name a key"),
                        ));
                    }
                }
                (url, *sizes)
            }
        };
        if !credential_free_https(url) {
            return Err(FieldError::new(
                at.field("url"),
                "[template.digests].url must be a credential-free HTTPS URL".to_string(),
            ));
        }
        if let Some(declared) = declared
            && declared != self.has_sizes()
        {
            return Err(FieldError::new(
                at.field("sizes"),
                format!(
                    "[template.digests].sizes = {declared} disagrees with the source, which \
                     carries {} — only `kind = \"json\"` with a `size_field` carries sizes, and a \
                     `{{size:FILE}}` variable needs one",
                    if self.has_sizes() {
                        "sizes"
                    } else {
                        "no sizes"
                    }
                ),
            ));
        }
        Ok(())
    }
}

fn validate_channel(channel: &ChannelRule, at: &FieldPath) -> Result<(), FieldError> {
    let (field, value) = match channel {
        ChannelRule::Truthy { name, .. } => ("name", name),
        ChannelRule::Value { prefix, .. } => ("prefix", prefix),
    };
    if !channel_name_is_valid(value) {
        return Err(FieldError::new(
            at.field(field),
            format!(
                "[[template.channel]].{field} `{value}` is not a channel name — {CHANNEL_NAME_RULE}"
            ),
        ));
    }
    if channel.from().trim().is_empty() {
        return Err(FieldError::new(
            at.field("from"),
            "[[template.channel]].from must name a field of the version record".to_string(),
        ));
    }
    Ok(())
}

/// An RFC 6901 JSON Pointer, with the empty pointer selecting the root and
/// nothing else a selector (PROP-075 `#TEMPLATE-SELECTOR`).
fn validate_pointer(select: &str, at: &FieldPath, prose: &str) -> Result<(), FieldError> {
    if select.is_empty() || select.starts_with('/') {
        return Ok(());
    }
    Err(FieldError::new(
        at.clone(),
        format!(
            "{prose} `{select}` is not an RFC 6901 JSON Pointer — a pointer is empty (the document \
             root) or begins with `/`; there is no JSONPath, no filter and no wildcard"
        ),
    ))
}
