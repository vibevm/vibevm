//! Which registries the effective profile allows — the line that makes
//! a missing `[source_policy] exclude_hosting` visible at the moment it
//! costs something.
//!
//! The failure an operator reads on a corporate network is almost never
//! about the registry it names: it is about the registry that *is not
//! being used*. A site has a mirror, the install walked past it to a
//! public host, and nothing in the message said so. The roster is the
//! declared, enabled `[[registry]]` set, captured when the catalog opens
//! a source and carried into the failure, so the answer travels with the
//! question.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use vibe_core::manifest::{HostingSelector, RegistrySection, RegistryType};

/// One registry as the failure names it: its alias, a one-word label for
/// what kind of thing it is, and whether it lives on a public hosting
/// family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryRosterRow {
    pub name: String,
    /// `monorepo` / `localdir` for the whole-repository and directory
    /// kinds; otherwise the hosting family (`github`, `gitverse`) or the
    /// bare host — whichever tells the reader where the bytes come from.
    pub label: String,
    /// Whether the row's url is on one of the public hosting families
    /// `[source_policy]` knows by name.
    pub public_host: bool,
    /// The row's lower-cased host, when its url has one.
    pub host: Option<String>,
}

/// The registries one failure may report, in declared order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistryRoster {
    rows: Vec<RegistryRosterRow>,
}

impl RegistryRoster {
    /// The roster a direct constructor gets: no catalog, nothing to say.
    ///
    /// ```
    /// use vibe_registry::git_failure::RegistryRoster;
    /// assert!(RegistryRoster::default().is_empty());
    /// assert_eq!(RegistryRoster::default().allowed_line(), None);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Build the roster from the declared `[[registry]]` rows; disabled
    /// rows are not allowed and do not appear.
    ///
    /// ```
    /// use vibe_core::manifest::RegistrySection;
    /// use vibe_registry::git_failure::RegistryRoster;
    ///
    /// let rows: Vec<RegistrySection> = toml::from_str::<toml::Value>(
    ///     "registry = [{ name = 'corp', url = 'https://git.example.invalid/vibe', type = 'monorepo' }, \
    ///      { name = 'vibespecs', url = 'https://github.com/vibespecs' }]",
    /// )
    /// .unwrap()["registry"]
    ///     .clone()
    ///     .try_into()
    ///     .unwrap();
    /// let roster = RegistryRoster::from_rows(&rows);
    /// assert_eq!(
    ///     roster.allowed_line().unwrap(),
    ///     "allowed registries: corp (monorepo), vibespecs (github)"
    /// );
    /// ```
    pub fn from_rows(rows: &[RegistrySection]) -> Self {
        RegistryRoster {
            rows: rows
                .iter()
                .filter(|row| row.enabled)
                .map(RegistryRosterRow::of)
                .collect(),
        }
    }

    /// `allowed registries: corp (monorepo), vibespecs (github)` — or
    /// `None` when nothing was declared to this registry instance.
    pub fn allowed_line(&self) -> Option<String> {
        if self.rows.is_empty() {
            return None;
        }
        let listed = self
            .rows
            .iter()
            .map(|row| format!("{} ({})", row.name, row.label))
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!("allowed registries: {listed}"))
    }

    /// The site registries — the declared rows that are *not* on a
    /// public hosting family, which is what a corporate mirror looks
    /// like from here.
    pub fn site_registries(&self) -> Vec<&RegistryRosterRow> {
        self.rows.iter().filter(|row| !row.public_host).collect()
    }

    /// Whether `host` is one of the declared rows' public hosts.
    pub fn is_public_host(&self, host: &str) -> bool {
        self.rows
            .iter()
            .any(|row| row.public_host && row.host.as_deref() == Some(host))
    }
}

impl RegistryRosterRow {
    fn of(row: &RegistrySection) -> Self {
        let kind = row.effective_type();
        let public_host = is_public_hosting(&row.url);
        RegistryRosterRow {
            name: row.name.clone(),
            label: label_of(kind, &row.url),
            public_host,
            host: host_of(&row.url),
        }
    }
}

/// The one-word label a roster row carries.
///
/// The two structural kinds name themselves — a `monorepo` is one
/// repository and a `localdir` is a directory, and neither fact is
/// readable off the url. For the per-package kinds the useful word is
/// *where*, so the hosting family (or the bare host) is the label.
fn label_of(kind: RegistryType, url: &str) -> String {
    match kind {
        RegistryType::Monorepo => "monorepo".to_string(),
        RegistryType::Localdir => "localdir".to_string(),
        RegistryType::Github => "github".to_string(),
        RegistryType::Gitverse => "gitverse".to_string(),
        RegistryType::Git => host_of(url).unwrap_or_else(|| "git".to_string()),
    }
}

/// Whether `url` is on a hosting family `[source_policy]` knows by name
/// — the public hosts a site mirror exists to replace.
pub(crate) fn is_public_hosting(url: &str) -> bool {
    [HostingSelector::Github, HostingSelector::Gitverse]
        .iter()
        .any(|family| family.matches_endpoint(url))
}

/// The normalised host of `url`, when it names a network endpoint.
pub(crate) fn host_of(url: &str) -> Option<String> {
    match vibe_core::remap::endpoint::observe_endpoint(url) {
        vibe_core::remap::endpoint::EndpointObservation::Network(authority) => Some(authority.host),
        _ => None,
    }
}
