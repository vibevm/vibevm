//! The version domain of a template package, and the channels derived
//! from its version records (PROP-075 `#TEMPLATE-RESOLUTION`,
//! `#TEMPLATE-CONCRETE-WINS`, `#CHANNEL-SYNTAX`).
//!
//! The domain is the **union** of the two sets — the versions the
//! template's source lists, and the concretely published versions of the
//! same package, minus the template revisions, which are not produced
//! versions. For a version in both, the published package is resolved and
//! fetched and **the template is not rendered for it at all** — not
//! rendered and compared, not rendered and discarded. That makes
//! hand-fixing one version free of any digest fetch and makes the fix
//! total.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CONCRETE-WINS");

use std::collections::{BTreeMap, BTreeSet};

use vibe_core::manifest::ChannelRule;
use vibe_core::{ChannelName, Group};
use vibe_template::VersionRecord;

use super::error::TemplateError;
use super::{ActiveTemplate, TemplateLayer};

/// Where one version of a template package comes from.
///
/// ```
/// use vibe_registry::templates::Origin;
///
/// assert_eq!(Origin::Template.label(), "template");
/// assert_eq!(Origin::Published.label(), "published");
/// assert_eq!(Origin::Overrides.label(), "published (overrides template)");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Produced by the template and not published concretely.
    Template,
    /// Published concretely and not produced by the template.
    Published,
    /// Published concretely **and** produced by the template: the
    /// published package wins and the template is never rendered for it.
    Overrides,
}

impl Origin {
    /// The word `vibe app versions` prints.
    pub fn label(self) -> &'static str {
        match self {
            Origin::Template => "template",
            Origin::Published => "published",
            Origin::Overrides => "published (overrides template)",
        }
    }
}

/// Every version of one template package, with each version's origin and
/// the channel table the template's rules produced.
#[derive(Debug, Clone, Default)]
pub struct VersionDomain {
    produced: BTreeSet<semver::Version>,
    published: BTreeSet<semver::Version>,
    records: BTreeMap<semver::Version, VersionRecord>,
    channels: BTreeMap<String, BTreeSet<semver::Version>>,
    skipped: Vec<String>,
}

impl VersionDomain {
    /// Every version, ascending — what [`crate::Registry::list_versions`]
    /// answers.
    ///
    /// ```
    /// use vibe_registry::templates::VersionDomain;
    ///
    /// let domain = VersionDomain::of_published(
    ///     ["1.0.0", "2.0.0"].iter().map(|v| v.parse().unwrap()).collect(),
    /// );
    /// assert_eq!(domain.all().len(), 2);
    /// ```
    pub fn all(&self) -> Vec<semver::Version> {
        self.produced
            .union(&self.published)
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// A domain of published versions alone — the answer when a package
    /// carries no template.
    pub fn of_published(published: BTreeSet<semver::Version>) -> Self {
        VersionDomain {
            published,
            ..VersionDomain::default()
        }
    }

    /// Where this version comes from, or `None` when the domain has no
    /// such version.
    pub fn origin(&self, version: &semver::Version) -> Option<Origin> {
        match (
            self.produced.contains(version),
            self.published.contains(version),
        ) {
            (true, true) => Some(Origin::Overrides),
            (true, false) => Some(Origin::Template),
            (false, true) => Some(Origin::Published),
            (false, false) => None,
        }
    }

    /// Whether this version must be **rendered** rather than resolved
    /// from the registry: produced by the template and not published.
    pub fn needs_render(&self, version: &semver::Version) -> bool {
        matches!(self.origin(version), Some(Origin::Template))
    }

    /// The version record a render of this version needs.
    pub fn record(&self, version: &semver::Version) -> Option<&VersionRecord> {
        self.records.get(version)
    }

    /// The resolved channel table: channel name → its members,
    /// ascending.
    pub fn channels(&self) -> &BTreeMap<String, BTreeSet<semver::Version>> {
        &self.channels
    }

    /// The source records this domain skipped, each with its reason — an
    /// upstream index legitimately carries shapes vibe does not
    /// understand, and a skip is reported rather than hidden.
    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }
}

impl TemplateLayer {
    /// The version domain of one template package: the source's versions
    /// unioned with the concretely published ones, the template
    /// revisions excluded, and the channel table computed from the
    /// template's `[[template.channel]]` rules.
    pub fn domain(&self, template: &ActiveTemplate) -> Result<VersionDomain, TemplateError> {
        let document = self.version_document(template)?;
        let (records, skipped) = vibe_template::version_records(
            &template.section,
            document.as_ref().map(|e| e.document.as_str()),
        )
        .map_err(|error| TemplateError::Declaration {
            package: template.coordinate(),
            reason: error.to_string(),
        })?;
        let mut domain = VersionDomain {
            published: template.published.iter().cloned().collect(),
            skipped,
            ..VersionDomain::default()
        };
        for record in records {
            domain.produced.insert(record.version.clone());
            domain.records.insert(record.version.clone(), record);
        }
        // A template revision is not a produced version: it is the
        // revision of the template itself (PROP-075 `#TEMPLATE-FLAG`),
        // and `discover_in_directory` already keeps the two apart — this
        // is the belt the fact asks for, so a revision that somehow
        // reached `published` cannot be installed as one.
        for revision in &template.revisions {
            domain.published.remove(revision);
        }
        for rule in &template.section.channels {
            apply_channel_rule(rule, &mut domain);
        }
        Ok(domain)
    }

    /// Resolve one channel of one template package to its members.
    pub fn channel_members(
        &self,
        template: &ActiveTemplate,
        channel: &ChannelName,
    ) -> Result<Option<Vec<semver::Version>>, TemplateError> {
        let domain = self.domain(template)?;
        Ok(domain
            .channels()
            .get(channel.as_str())
            .map(|members| members.iter().cloned().collect()))
    }
}

/// One `[[template.channel]]` row applied to every record of a domain.
fn apply_channel_rule(rule: &ChannelRule, domain: &mut VersionDomain) {
    let field = rule.from().to_string();
    let memberships: Vec<(String, semver::Version)> = domain
        .records
        .iter()
        .filter_map(|(version, record)| {
            let value = record.meta.get(&field)?;
            match rule {
                ChannelRule::Truthy { name, .. } => {
                    value.is_truthy().then(|| (name.clone(), version.clone()))
                }
                // One channel per distinct value, named `<prefix>/<value
                // lower-cased>`. A value that is not truthy names no
                // channel: Node's `lts: false` must not produce
                // `lts/false`.
                ChannelRule::Value { prefix, .. } => value.is_truthy().then(|| {
                    (
                        format!("{prefix}/{}", value.to_string().to_ascii_lowercase()),
                        version.clone(),
                    )
                }),
            }
        })
        .collect();
    for (name, version) in memberships {
        if ChannelName::parse(&name).is_none() {
            // A derived name that is not a channel name is dropped with
            // a warning rather than invented into existence: the rule is
            // the package's, the grammar is vibe's.
            tracing::warn!(
                target: "vibe_registry::templates",
                "a [[template.channel]] rule derived `{name}`, which is not a channel name; ignored"
            );
            continue;
        }
        domain.channels.entry(name).or_default().insert(version);
    }
}

/// A [`vibe_core::channel::ChannelResolver`] over one already-discovered
/// template, so `lower_channel` works against it with no knowledge of the
/// registry layer.
pub struct TemplateChannels<'a> {
    layer: &'a TemplateLayer,
    template: &'a ActiveTemplate,
}

impl<'a> TemplateChannels<'a> {
    /// Bind a layer and a template as one channel resolver.
    pub fn new(layer: &'a TemplateLayer, template: &'a ActiveTemplate) -> Self {
        TemplateChannels { layer, template }
    }
}

impl vibe_core::channel::ChannelResolver for TemplateChannels<'_> {
    fn domain(&self, _group: &Group, _name: &str) -> Result<Vec<semver::Version>, String> {
        self.layer
            .domain(self.template)
            .map(|domain| domain.all())
            .map_err(|error| error.to_string())
    }

    fn members(
        &self,
        _group: &Group,
        _name: &str,
        channel: &ChannelName,
    ) -> Result<Option<Vec<semver::Version>>, String> {
        self.layer
            .channel_members(self.template, channel)
            .map_err(|error| error.to_string())
    }
}
