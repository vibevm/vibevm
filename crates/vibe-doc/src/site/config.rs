//! `site.toml` — the two sources and the one table
//! (PROP-057 `##SITE-TWO-SOURCES`, `##SITE-SOURCE-REGISTRY`,
//! `##SITE-SOURCE-HOST`, `##SITE-HOST-CHECKOUT`, `##SITE-HOST-POLL`).
//!
//! The file is read by the generated type and nothing else: the schema
//! `schemas/doc_site_config.jtd.json` is the contract, and a hand-written
//! twin of it would be a second opinion about what a configuration means
//! (PROP-044, campaign rule R-04). What this module adds is the layer the
//! wire cannot carry — the DEFAULTS, resolved once, so that no reader of
//! a configuration has to know that an absent `ref` means `main`.
//!
//! ## Why the defaults live here and not in the schema
//!
//! JTD states which members may be absent; it cannot state what an absent
//! member means. Put the meaning in each reader and there are as many
//! answers as readers; put it here and `##SITE-HOST-POLL`'s «once an
//! hour» is written down once, beside the fact that it is a recommended
//! default awaiting the owner's word (campaign fork F-35) and not a
//! constant of the product.
//!
//! ## Values, not constants
//!
//! Every address in this file is a default a deployment may replace: the
//! registry, the host repository, the origin. A fork builds this same
//! source for a domain that is not vibevm.org, and a test builds it for a
//! directory. What is NOT configurable is anything that would authorise
//! something: there is no `auth`, no token and no env name in the shape,
//! because the site reads every source anonymously and a configuration
//! that COULD carry a credential invites one onto a renderer that has no
//! use for it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-TWO-SOURCES");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vibe_core::manifest::NamingConvention;
use vibe_wire::generated::doc_site_config::DocSiteConfig;

use crate::error::{DocError, Result};

/// The schema number a file written today carries, and the only one this
/// build reads.
pub const SCHEMA_VERSION: u32 = 1;

/// The file's conventional name, so a container and a report spell it the
/// same way.
pub const FILENAME: &str = "site.toml";

/// The registry the site reads when the configuration names none by
/// address — the default of `##SITE-SOURCE-REGISTRY`.
pub const DEFAULT_REGISTRY_URL: &str = "https://github.com/vibespecs";

/// The host repository of `##SITE-SOURCE-HOST`. An identity: the builder
/// reads the checkout on disk and never clones this.
pub const DEFAULT_HOST_GIT: &str = "https://github.com/vibevm/vibevm";

/// The branch whose current state is the host's release
/// (`##SITE-SOURCE-HOST`: the host has no release tags).
pub const DEFAULT_HOST_REF: &str = "main";

/// How long a successful host render is left alone, however often the
/// branch moves. The recommended default of `##SITE-HOST-POLL`, pending
/// the owner's word on the frequency (campaign fork F-35); the phase-0
/// measurement of the render cost is what makes an hour comfortable
/// rather than arbitrary.
pub const DEFAULT_DEBOUNCE_MINUTES: u32 = 60;

/// The domain the site is built for when the configuration names none.
pub const DEFAULT_ORIGIN: &str = "https://vibevm.org";

/// One package registry, in the form of a `[[registry]]` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    /// The local alias, as in `[[registry]].name`.
    pub name: String,
    /// The organisation-root URL, as in `[[registry]].url`.
    pub url: String,
    /// Where this registry's index lives, as in `[[registry]].index_url`.
    /// `None` means the default `<url>/index`; the exact value `none`
    /// switches the index off and is kept as it was written, because the
    /// resolution ladder — not this reader — is what knows that spelling
    /// (`vibe_registry::index_client::resolve_index_url`).
    pub index_url: Option<String>,
    /// The convention mapping a coordinate to a repository name.
    pub naming: NamingConvention,
}

impl Registry {
    /// This registry as the `[[registry]]` section it is shaped after.
    ///
    /// It exists so the index-location ladder
    /// (`vibe_registry::index_client::resolve_index_url`) can read it: the
    /// site must ask WHERE an index lives through the one law that
    /// answers that, or it would poll a different address than
    /// `vibe install` does on the same machine.
    ///
    /// Everything that authenticates comes out at its own default,
    /// because the site has nothing to authenticate with: `auth = none`
    /// is public read, which is exactly what a documentation site does.
    pub fn as_section(&self) -> vibe_core::manifest::RegistrySection {
        vibe_core::manifest::RegistrySection {
            name: self.name.clone(),
            url: self.url.clone(),
            r#ref: DEFAULT_REGISTRY_REF.to_string(),
            naming: self.naming,
            auth: vibe_core::manifest::AuthKind::None,
            token_env: None,
            enabled: true,
            index_url: self.index_url.clone(),
        }
    }
}

/// The registry-level ref the ladder reads when it translates a public
/// GitHub organisation into its raw index. The same default a
/// `[[registry]]` block takes when it declares none.
pub const DEFAULT_REGISTRY_REF: &str = "main";

/// The host's source repository, read as a checkout on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    /// Which repository the checkout is of. Identity only: the builder
    /// never clones it, because the host root is a `[project]` and not a
    /// publishable package (`##SITE-HOST-CHECKOUT`).
    pub git: String,
    /// The branch whose current state is rendered.
    pub git_ref: String,
    /// The directory the deploy keeps the checkout in, absolute.
    pub checkout: PathBuf,
    /// How long a successful render of the host is left alone.
    pub debounce_minutes: u32,
}

/// Which theme a reader who has expressed no preference gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// The reader's own operating-system setting — where campaign fork
    /// F-48 leaves the question until the design review.
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    /// The word a configuration spells.
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Every spelling, in the order a refusal lists them.
    pub const ALL: &'static [Theme] = &[Theme::System, Theme::Light, Theme::Dark];

    /// Read a configured value.
    pub fn parse(text: &str) -> Option<Theme> {
        Theme::ALL.iter().copied().find(|t| t.as_str() == text)
    }
}

/// The first-party analytics property (D-24, `##SITE-ANALYTICS`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Analytics {
    /// The property id. Empty means «render no tag», which is the
    /// instruction and not a hole: a tag with an empty attribute loads
    /// the script and reports nothing.
    pub website_id: String,
    /// Where the tag reports to — the site's own origin unless the
    /// configuration names another.
    pub host_url: String,
}

impl Analytics {
    /// Does this build publish a tag at all?
    pub fn is_silent(&self) -> bool {
        self.website_id.trim().is_empty()
    }
}

/// The configuration with every default resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// Where this was read from — named in every report, so an operator
    /// sees which file a number came from.
    pub path: PathBuf,
    /// The registries, in the order they are polled.
    pub registries: Vec<Registry>,
    /// The host, when this render carries one.
    pub host: Option<Host>,
    /// The path documentation is mounted at, with a leading and a
    /// trailing slash — the form [`crate::content::SITE_BASE`] has,
    /// because it is the same value and two spellings of one base is how
    /// a link comes out with a double slash in it.
    pub base: String,
    /// Scheme and host, no trailing slash.
    pub origin: String,
    pub default_theme: Theme,
    /// The documentations the front of the site shows first, as
    /// `<group>/<name>` coordinates, in the order the configuration
    /// wrote them. Empty means «feature nothing», which is what a
    /// deployment that has made no editorial choice wants.
    pub featured: Vec<String>,
    pub analytics: Analytics,
}

impl Site {
    /// Read and resolve `site.toml`.
    ///
    /// ```no_run
    /// let site = vibe_doc::site::Site::read(std::path::Path::new("site.toml")).unwrap();
    /// assert!(site.base.starts_with('/') && site.base.ends_with('/'));
    /// ```
    pub fn read(path: &Path) -> Result<Site> {
        let text = std::fs::read_to_string(path).map_err(|e| DocError::io("reading", path, e))?;
        let parsed: DocSiteConfig = toml::from_str(&text).map_err(|e| DocError::Site {
            message: format!("`{}` does not parse: {e}", path.display()),
        })?;
        Site::resolve(path, parsed)
    }

    /// Apply the defaults to a parsed document.
    ///
    /// Separate from [`Site::read`] so a test states a configuration
    /// rather than writing one to disk, and so the one place that knows
    /// what an absent member means is reachable without a file.
    pub fn resolve(path: &Path, parsed: DocSiteConfig) -> Result<Site> {
        if parsed.schema != SCHEMA_VERSION {
            return Err(DocError::Site {
                message: format!(
                    "`{}` declares `schema = {}`, and this build reads {SCHEMA_VERSION}",
                    path.display(),
                    parsed.schema
                ),
            });
        }
        // A relative path in a configuration is relative to the
        // configuration, never to whatever directory the renderer was
        // started in: the file and the checkout beside it travel together
        // into an image, and the working directory does not.
        let beside = path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let sources = parsed
            .source
            .unwrap_or(vibe_wire::generated::doc_site_config::SiteSources {
                host: None,
                registry: Vec::new(),
            });

        let mut registries = Vec::new();
        for declared in sources.registry {
            let naming = match &declared.naming {
                Some(word) => naming_convention(path, word)?,
                None => NamingConvention::default(),
            };
            registries.push(Registry {
                name: declared.name,
                url: declared.url.trim_end_matches('/').to_string(),
                index_url: declared.index_url,
                naming,
            });
        }

        let host = sources.host.map(|declared| Host {
            git: declared.git,
            git_ref: declared
                .ref_
                .unwrap_or_else(|| DEFAULT_HOST_REF.to_string()),
            checkout: absolute(&beside, declared.checkout.as_deref()),
            debounce_minutes: declared
                .debounce_minutes
                .unwrap_or(DEFAULT_DEBOUNCE_MINUTES),
        });

        // Nothing to render is a configuration mistake and not an empty
        // site: a builder that answered it with an empty output would
        // deploy the absence over the previous render.
        if registries.is_empty() && host.is_none() {
            return Err(DocError::Site {
                message: format!(
                    "`{}` names no source: the site reads a package registry \
                     (`[[source.registry]]`), the host's checkout (`[source.host]`), \
                     or both",
                    path.display()
                ),
            });
        }

        let table = parsed
            .site
            .unwrap_or(vibe_wire::generated::doc_site_config::SiteTable {
                analytics: None,
                base_path: None,
                default_theme: None,
                featured: Vec::new(),
                origin: None,
            });
        let base = mount(table.base_path.as_deref());
        let origin = match table.origin {
            Some(origin) => {
                let trimmed = origin.trim_end_matches('/').to_string();
                if trimmed.is_empty() {
                    return Err(DocError::Site {
                        message: format!(
                            "`{}` declares an empty `[site].origin`, and `canonical`, \
                             `hreflang` and the sitemap are absolute addresses by \
                             specification — a page built on nothing claims to live at `/`",
                            path.display()
                        ),
                    });
                }
                trimmed
            }
            None => DEFAULT_ORIGIN.to_string(),
        };
        let default_theme = match &table.default_theme {
            Some(word) => Theme::parse(word).ok_or_else(|| DocError::Site {
                message: format!(
                    "`{}` declares `[site].default_theme = \"{word}\"`, and the themes \
                     are {}",
                    path.display(),
                    spellings(Theme::ALL.iter().map(|t| t.as_str()))
                ),
            })?,
            None => Theme::default(),
        };
        let declared =
            table
                .analytics
                .unwrap_or(vibe_wire::generated::doc_site_config::SiteAnalytics {
                    host_url: None,
                    website_id: None,
                });
        let analytics = Analytics {
            website_id: declared.website_id.unwrap_or_default().trim().to_string(),
            // A first-party tag served by the domain itself reports to
            // the domain itself, so the origin is the answer and not a
            // guess at one.
            host_url: declared
                .host_url
                .map(|u| u.trim_end_matches('/').to_string())
                .unwrap_or_else(|| origin.clone()),
        };

        // The coordinates are kept in the order they were written — a
        // featured list is an editorial ranking and not a set — and a
        // blank entry is dropped rather than carried into a comparison
        // nothing can ever match.
        let featured: Vec<String> = table
            .featured
            .iter()
            .map(|coordinate| coordinate.trim().to_string())
            .filter(|coordinate| !coordinate.is_empty())
            .collect();

        Ok(Site {
            path: path.to_path_buf(),
            registries,
            host,
            base,
            origin,
            default_theme,
            featured,
            analytics,
        })
    }

    /// The featured coordinates no source publishes.
    ///
    /// A warning and never a refusal: this file is authored beside a
    /// deploy and the registry moves without it, so a package that was
    /// renamed yesterday must not stop today's render. Saying it out
    /// loud in the run is what keeps the list from quietly featuring
    /// nothing.
    pub fn featured_absent<'a>(&'a self, published: &BTreeSet<String>) -> Vec<&'a str> {
        self.featured
            .iter()
            .filter(|coordinate| !published.contains(*coordinate))
            .map(String::as_str)
            .collect()
    }

    /// The configuration as an operator reads it back — every value,
    /// including the ones that came from a default.
    ///
    /// This is the whole defence against a misspelled key. The generated
    /// reader is permissive, like every generated reader in this tree, so
    /// `[souce.host]` is not a refusal; it is a host that is not there.
    /// Printing what was RESOLVED says so in the first line of the run,
    /// and says it more usefully than a refusal would, because it also
    /// shows the values nobody wrote down.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("site: {}\n", self.path.display()));
        for registry in &self.registries {
            out.push_str(&format!(
                "  registry {} {} (index {}, naming {})\n",
                registry.name,
                registry.url,
                registry.index_url.as_deref().unwrap_or("<url>/index"),
                naming_word(registry.naming),
            ));
        }
        if self.registries.is_empty() {
            out.push_str("  registry none\n");
        }
        match &self.host {
            Some(host) => out.push_str(&format!(
                "  host {} @{} from {} (debounce {} min)\n",
                host.git,
                host.git_ref,
                host.checkout.display(),
                host.debounce_minutes,
            )),
            None => out.push_str("  host none\n"),
        }
        out.push_str(&format!(
            "  mounted at {} on {}, theme {}\n",
            self.base,
            self.origin,
            self.default_theme.as_str()
        ));
        out.push_str(&format!(
            "  featured {}\n",
            if self.featured.is_empty() {
                "none".to_string()
            } else {
                self.featured.join(", ")
            }
        ));
        out.push_str(&format!(
            "  analytics {}\n",
            if self.analytics.is_silent() {
                "none — no website id, so no tag is rendered".to_string()
            } else {
                format!("reporting to {}", self.analytics.host_url)
            }
        ));
        out
    }
}

/// The mount path in the one spelling the rest of the pipeline uses: a
/// leading slash and a trailing slash. `/doc`, `doc/` and `/doc/` are one
/// value written three ways, and a base without the trailing slash is how
/// a link comes out as `/docorg.vibevm.core/…`.
fn mount(declared: Option<&str>) -> String {
    let Some(text) = declared else {
        return crate::content::SITE_BASE.to_string();
    };
    let trimmed = text.trim().trim_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    format!("/{trimmed}/")
}

/// A configured path, made absolute against the directory the
/// configuration sits in.
fn absolute(beside: &Path, declared: Option<&str>) -> PathBuf {
    let Some(text) = declared else {
        return beside.to_path_buf();
    };
    let path = PathBuf::from(text);
    if path.is_absolute() {
        return path;
    }
    beside.join(path)
}

/// Read a `naming` value through the one vocabulary that owns it.
///
/// The value goes back through `NamingConvention`'s own serde spellings
/// rather than a table written here: `kind/name` and `kind-name` are the
/// registry's words, and a second table of them in this file would be a
/// second law about what a repository is called.
fn naming_convention(path: &Path, word: &str) -> Result<NamingConvention> {
    toml::Value::String(word.to_string())
        .try_into()
        .map_err(|_| DocError::Site {
            message: format!(
                "`{}` declares `naming = \"{word}\"`, and the conventions are {}",
                path.display(),
                spellings(["fqdn", "kind-name", "name", "kind/name"]),
            ),
        })
}

/// The word a `naming` convention is written as, asked of the enum's own
/// serde spelling for the reason [`naming_convention`] reads it that way:
/// one vocabulary, read and written from the same place. The fallback is
/// unreachable — a fieldless enum always serialises — and it is a word
/// rather than a panic because a report is not worth a crash.
fn naming_word(naming: NamingConvention) -> String {
    toml::Value::try_from(naming)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "?".to_string())
}

/// A list of accepted spellings, as a refusal prints them.
fn spellings<'a>(words: impl IntoIterator<Item = &'a str>) -> String {
    words
        .into_iter()
        .map(|w| format!("`{w}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
