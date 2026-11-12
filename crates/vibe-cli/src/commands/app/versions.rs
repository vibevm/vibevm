//! `vibe app versions PACKAGE|SLOT`: one listing over a package's whole
//! version domain (PROP-075 `#VERSIONS-LIST`, `#VERSIONS-LIST-OUTPUT`).
//!
//! The library in `vibe-app` joins a domain with this home's index; this
//! adapter supplies the domain and draws the rows. The domain is the
//! **skeleton** stage of PROP-075 `#TEMPLATE-TWO-STAGE`: a template's
//! version list and nothing else — no digest document, no render, no
//! installation. Inside the version-list TTL, under `--offline`, and
//! under `--installed` it costs no request at all.
//!
//! The `in drop` mark of PROP-066 `#drops` is the second thing this
//! adapter supplies, in `versions/drop.rs`, and it is held to the same
//! budget: the file names come out of the skeleton render the domain read
//! already produced, and the drop is asked by name alone.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT");

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use vibe_app::{
    DirectoryRoots, Domain, DomainVersion, Domains, Origin, SettingsRoot, TemplateFacts,
    VersionsError,
};
use vibe_core::Group;
use vibe_registry::templates::{ActiveTemplate, DirectoryTemplate, TemplateLayer};
use vibe_wire::generated::application_versions as wire;

use super::global;
use crate::cli::AppVersionsArgs;
use crate::output;

#[path = "versions/drop.rs"]
mod in_drop;

pub(super) fn run(
    ctx: &output::Context,
    home: &SettingsRoot,
    mut args: AppVersionsArgs,
    embedded_root: Option<&Path>,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<()> {
    // `PACKAGE|SLOT` in three tiers (PROP-059 `#SHORT-NAME-SLOT-PRECEDENCE`):
    // the first two are below, in the library; a word that is neither a
    // slot of this home nor an installed package is resolved here, once,
    // through the same registries the domain is read from.
    args.target = super::short_name::versions_target(
        ctx,
        home,
        &args.target,
        super::short_name::Inputs {
            registry: args.registry.as_deref(),
            embedded: embedded_root,
            offline,
            operation,
        },
        args.installed,
    )?;
    let index = vibe_app::Index::load(home)?;
    // Every configured `[artifacts] packages` drop, highest layer first;
    // a version found in any of them is marked (PROP-066 `#drops`). With
    // none configured nothing is marked and nothing is an error — but
    // `--in-drop` asked for a narrowing the home cannot perform, and that
    // refuses by name rather than answering an empty listing.
    let drops: Vec<PathBuf> = vibe_core::remap::active()
        .package_drops()
        .iter()
        .map(|path| path.to_path_buf())
        .collect();
    if args.in_drop && drops.is_empty() {
        return Err(vibe_app::AppError::Versions(VersionsError::NoDrop).into());
    }
    let domains = RegistryDomains {
        settings: global::settings_root()?,
        registry: args.registry.clone(),
        embedded: embedded_root.map(Path::to_path_buf),
        offline,
        operation: operation.clone(),
        // `--installed` answers from this home and the template cache
        // alone, which is what makes it free of a registry and of the
        // network (PROP-075 `#VERSIONS-LIST-OUTPUT`).
        cache_only: args.installed,
        drops,
        found: RefCell::default(),
    };
    let request = vibe_app::VersionsRequest {
        target: &args.target,
        pattern: args.pattern.as_deref(),
        channel: args.channel.as_deref(),
        installed: args.installed,
        in_drop: args.in_drop,
        limit: args.limit,
        all: args.all,
    };
    let listing =
        ctx.suspend_progress(|| vibe_app::versions_listing(&index, &request, &domains))?;
    if ctx.is_json() {
        return ctx.emit_json(&listing);
    }
    // The policy belongs in the trailer, beside the slot and the default,
    // because it is what decides which of the rows above an install would
    // actually land on (PROP-075 `#VERSIONS-SELECT`).
    print(
        &listing,
        ctx.is_quiet(),
        super::version_select::effective()?.0,
    );
    Ok(())
}

/// The version domain of one package, read through the same source chain
/// an install resolves through.
///
/// Shared with the version-selection policy of PROP-075
/// `#VERSIONS-SELECT`, which needs exactly these two answers over exactly
/// this source chain — the domain a requirement chooses within, and the
/// `in drop` probe that says which of its versions this machine can
/// install. Reusing the value rather than restating it is what keeps the
/// listing's `in drop` mark and the policy's verdict from ever disagreeing
/// (`app/version_select.rs`).
pub(crate) struct RegistryDomains {
    settings: SettingsRoot,
    registry: Option<PathBuf>,
    embedded: Option<PathBuf>,
    offline: bool,
    operation: vibe_package_source::SourceOperation,
    cache_only: bool,
    /// Every configured `[artifacts] packages` drop, highest layer first;
    /// empty when the home declares none, and then nothing is marked.
    drops: Vec<PathBuf>,
    /// What the `in drop` probe needs, per listed package, captured by
    /// the domain read that already had it in hand (`versions/drop.rs`).
    found: RefCell<BTreeMap<String, Found>>,
}

impl RegistryDomains {
    /// The domain reader of one command, over the source chain that
    /// command resolves through. `drops` is every configured
    /// `[artifacts] packages` drop, highest layer first; with none,
    /// nothing is ever marked `in drop`.
    pub(crate) fn new(
        settings: SettingsRoot,
        registry: Option<PathBuf>,
        embedded: Option<PathBuf>,
        offline: bool,
        operation: vibe_package_source::SourceOperation,
        drops: Vec<PathBuf>,
    ) -> Self {
        RegistryDomains {
            settings,
            registry,
            embedded,
            offline,
            operation,
            cache_only: false,
            drops,
            found: RefCell::default(),
        }
    }
}

/// One listed package's half of the drop probe's inputs, kept from the
/// domain read.
///
/// It holds the lease on purpose: a `monorepo` root is a capture its
/// source owns and lives exactly as long as that source (PROP-075
/// `#VERSIONS-CHANNEL-DISCOVERY`), and the probe reads under those roots
/// row by row, after `domain` has returned.
struct Found {
    /// The package's template with its domain, when a directory root
    /// carries one — the skeleton render of a produced version comes from
    /// here, with no request and no second registry walk.
    template: Option<DirectoryTemplate>,
    /// The directory roots the domain was read over, with their lease —
    /// where a concretely published version's own manifest is read from.
    roots: DirectoryRoots,
}

impl Domains for RegistryDomains {
    fn domain(&self, group: &str, name: &str) -> Result<Domain, String> {
        let parsed = Group::parse(group).map_err(|error| error.to_string())?;
        let Some(layer) = vibe_registry::templates::active() else {
            // No layer means no template cache and no template
            // discovery: the pre-PROP-075 answer, which is the published
            // versions alone.
            return self.published(&parsed, name);
        };
        if self.cache_only {
            return Ok(cached_domain(&layer, &parsed, name));
        }
        // The whole value is held for the loop: a `monorepo` root is a
        // capture its source owns, and dropping the lease would delete the
        // tree out from under the read (PROP-075
        // `#VERSIONS-CHANNEL-DISCOVERY`).
        let found_roots = self.roots()?;
        let mut hit = None;
        for (at, root) in found_roots.roots().iter().enumerate() {
            match layer.for_directory(root, &parsed, name) {
                Ok(Some(found)) => {
                    hit = Some((at, found));
                    break;
                }
                Ok(None) => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        if let Some((at, found)) = hit {
            let root = found_roots.roots()[at].clone();
            let domain = directory_domain(&layer, &found, Some(root.as_path()));
            self.keep(
                group,
                name,
                Found {
                    template: Some(found),
                    roots: found_roots,
                },
            );
            return Ok(domain);
        }
        // The one registry shape with no directory root: an indexed
        // per-package registry, which reads its template through the
        // index record and one file at the active revision's tag
        // (PROP-075 `#TEMPLATE-IN-A-REPO`). Before it existed this
        // listing fell straight through to the published versions, so a
        // template package on a public registry answered no channels, no
        // origins and no provenance (`BACKLOG.md#b-269`). The drop probe
        // has no directory template to render here, so such rows carry no
        // `in drop` mark.
        if let Some(found) = self.repo_template(&parsed, name)? {
            let domain = repo_domain(&layer, &found);
            self.keep(
                group,
                name,
                Found {
                    template: None,
                    roots: found_roots,
                },
            );
            return Ok(domain);
        }
        // No template under any root; the roots are still kept, because a
        // version the registry publishes concretely has its own manifest
        // under one of them and the probe reads it there.
        let published = self.published(&parsed, name);
        self.keep(
            group,
            name,
            Found {
                template: None,
                roots: found_roots,
            },
        );
        published
    }

    fn in_drop(&self, group: &str, name: &str, version: &semver::Version) -> Option<Vec<String>> {
        self.drop_paths(group, name, version)
    }
}

impl RegistryDomains {
    /// Record what the drop probe will ask of this package.
    fn keep(&self, group: &str, name: &str, found: Found) {
        self.found
            .borrow_mut()
            .insert(format!("{group}/{name}"), found);
    }

    /// The directory-shaped registries this listing may read a template
    /// from, in precedence order (PROP-075 `#TEMPLATE-DISCOVERY`), with
    /// the lease that keeps a captured working copy on disk.
    fn roots(&self) -> Result<vibe_app::DirectoryRoots, String> {
        if let Some(root) = &self.registry {
            return Ok(vibe_app::DirectoryRoots::given(vec![root.clone()]));
        }
        vibe_app::directory_roots_from_global_with_operation(
            self.settings.path(),
            self.offline,
            &self.operation,
        )
        .map(|found| found.ahead_of(self.embedded.clone()))
        .map_err(|error| error.to_string())
    }

    /// The template an indexed per-package registry carries, if any.
    ///
    /// `--registry DIR` names a directory registry, which the roots loop
    /// above already read, so this asks nothing in that case.
    fn repo_template(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<vibe_registry::templates::RepoTemplate>, String> {
        if self.registry.is_some() {
            return Ok(None);
        }
        vibe_app::template_from_global_with_operation(
            self.settings.path(),
            group,
            name,
            self.offline,
            &self.operation,
        )
        .map_err(|error| error.to_string())
    }

    /// The concretely published versions of a package with no template.
    fn published(&self, group: &Group, name: &str) -> Result<Domain, String> {
        if let Some(root) = &self.registry {
            let local = crate::registry::application_local_registry(root.clone())
                .map_err(|error| format!("{error:#}"))?;
            let versions = match local.list_versions(group, name) {
                Ok(versions) => versions,
                Err(vibe_registry::RegistryError::UnknownPackage { .. }) => Vec::new(),
                Err(error) => return Err(error.to_string()),
            };
            return Ok(Domain::of_published(versions));
        }
        vibe_app::versions_from_global_with_operation(
            self.settings.path(),
            group,
            name,
            self.offline,
            &self.operation,
        )
        .map(Domain::of_published)
        .map_err(|error| error.to_string())
    }
}

/// The domain of a template found on disk, with every version's origin
/// and the channel table its rules produced.
fn directory_domain(
    layer: &TemplateLayer,
    found: &DirectoryTemplate,
    registry_root: Option<&std::path::Path>,
) -> Domain {
    let versions = found
        .domain
        .all()
        .into_iter()
        .map(|version| DomainVersion {
            origin: found.domain.origin(&version).unwrap_or(Origin::Published),
            version,
        })
        .collect();
    Domain {
        versions,
        channels: found.domain.channels().clone(),
        template: Some(template_facts(layer, &found.template, registry_root)),
        notices: found.domain.skipped().to_vec(),
    }
}

/// The domain of a template an indexed per-package registry carries.
///
/// The same drawing as [`directory_domain`], and deliberately so: the two
/// registry shapes differ in how the template is **found**, never in what
/// a listing says about it. There is no registry root, so no mirror
/// snapshot is read — a per-package registry carries none.
fn repo_domain(layer: &TemplateLayer, found: &vibe_registry::templates::RepoTemplate) -> Domain {
    let versions = found
        .domain
        .all()
        .into_iter()
        .map(|version| DomainVersion {
            origin: found.domain.origin(&version).unwrap_or(Origin::Published),
            version,
        })
        .collect();
    Domain {
        versions,
        channels: found.domain.channels().clone(),
        template: Some(template_facts(layer, &found.template, None)),
        notices: found.domain.skipped().to_vec(),
    }
}

/// The domain read from the template cache alone — no registry root, no
/// index read, no request (PROP-075 `#ACTIVATION-HOOK`'s budget, which
/// `--installed` holds itself to as well).
fn cached_domain(layer: &TemplateLayer, group: &Group, name: &str) -> Domain {
    let Ok(Some(template)) = layer.cached_template(group, name) else {
        return Domain::default();
    };
    let channels = layer.cached_channels(group, name).unwrap_or_default();
    Domain {
        versions: Vec::new(),
        channels,
        // Read from the cache alone, so there is no registry to read a
        // snapshot's sync time out of; the field stays absent rather
        // than guessing from the primed bytes, which carry upstream's
        // own `fetched_at` and not the mirror's.
        template: Some(template_facts(layer, &template, None)),
        notices: Vec::new(),
    }
}

fn template_facts(
    layer: &TemplateLayer,
    template: &ActiveTemplate,
    registry_root: Option<&std::path::Path>,
) -> TemplateFacts {
    TemplateFacts {
        coordinate: template.coordinate(),
        revision: template.revision.to_string(),
        version_list_url: template.section.version_list_url().map(str::to_owned),
        fetched_at: layer
            .cache(&template.group, &template.name)
            .read_version_list()
            .ok()
            .flatten()
            .map(|envelope| envelope.fetched_at),
        // In a closed network the domain came from a mirror's snapshot,
        // and how old that snapshot is is the fact a user needs to decide
        // whether to ask the operator to re-run the mirror
        // (PROP-075 `#MIRROR-SNAPSHOT-LAYOUT`).
        snapshot_synced: registry_root.and_then(|root| {
            layer
                .registry_snapshot(root, &template.group, &template.name)
                .map(|snapshot| snapshot.synced().to_string())
        }),
    }
}

/// The human listing: one block per package, the rows newest first, and
/// the slot's trailer.
fn print(
    listing: &wire::ApplicationVersions,
    quiet: bool,
    policy: vibe_core::version_select::VersionSelect,
) {
    for package in &listing.packages {
        println!("{}: {} version(s)", package.package, package.total);
        if !quiet {
            for row in &package.versions {
                println!("  {}", row_line(row));
            }
            if package.more > 0 {
                println!("  … and {} more (`--all` shows every one)", package.more);
            }
            for notice in &package.notices {
                eprintln!("vibe app versions: {notice}");
            }
        }
    }
    println!("{}", trailer(listing, policy));
}

/// One row: the version, its origin, the channels it belongs to, what
/// this home holds of it and whether a drop already holds its payload.
///
/// The two notes share **one** trailing `— …` segment, comma-separated,
/// so every row has the same shape whichever notes it carries. The drop
/// mark is shown on an installed row too: it is a fact about the drop,
/// not about installability, and `--in-drop` means the same thing on
/// every row because of it.
fn row_line(row: &wire::Row) -> String {
    let mut line = format!("{} ({})", row.version, origin_label(&row.origin));
    if !row.channels.is_empty() {
        line.push_str(&format!(" [{}]", row.channels.join(", ")));
    }
    let notes: Vec<&str> = state_label(&row.state)
        .into_iter()
        .chain((!row.drop.is_empty()).then_some("in drop"))
        .collect();
    if !notes.is_empty() {
        line.push_str(&format!(" — {}", notes.join(", ")));
    }
    line
}

fn origin_label(origin: &wire::Origin) -> &'static str {
    match origin {
        wire::Origin::Template => Origin::Template.label(),
        wire::Origin::Published => Origin::Published.label(),
        wire::Origin::Overrides => Origin::Overrides.label(),
    }
}

fn state_label(state: &wire::State) -> Option<&'static str> {
    match state {
        wire::State::Available => None,
        wire::State::Installed => Some("installed"),
        wire::State::InstalledDefault => Some("installed (default)"),
        wire::State::InstalledHidden => Some("installed (hidden)"),
        wire::State::InstalledHeld => Some("installed (held)"),
    }
}

/// The trailer: the slot, the default, the mirror's sync time — and the
/// version-selection policy, which is the fact that turns this listing
/// from "what exists" into "what an install here would pick".
///
/// Under `drop-only` the rows are **not** filtered. A listing's job is to
/// show the domain, and `--in-drop` is the flag that narrows it; hiding
/// the unselectable rows would also hide the version an operator has to
/// carry into the drop, which is the one thing the policy's own refusal
/// goes out of its way to name. The trailer says which rows are
/// selectable instead.
fn trailer(
    listing: &wire::ApplicationVersions,
    policy: vibe_core::version_select::VersionSelect,
) -> String {
    let mut parts = Vec::new();
    match &listing.slot {
        Some(slot) => parts.push(format!("slot `{slot}`")),
        None => parts.push(format!("package `{}`", listing.target)),
    }
    parts.push(format!(
        "default {}",
        listing.default.as_deref().unwrap_or("(none)")
    ));
    if listing.off {
        parts.push(
            "the slot is off: its commands fall through to the system program, and \
             `vibe app use <slot> --default` turns it on"
                .to_owned(),
        );
    }
    if let Some(synced) = &listing.snapshot_synced {
        parts.push(format!("mirror snapshot synced {synced}"));
    }
    parts.push(match policy {
        vibe_core::version_select::VersionSelect::Newest => "versions select newest".to_owned(),
        vibe_core::version_select::VersionSelect::PreferDrop => {
            "versions select prefer-drop: a requirement picks the newest `in drop` row it \
             matches, else the newest"
                .to_owned()
        }
        vibe_core::version_select::VersionSelect::DropOnly => {
            "versions select drop-only: only an `in drop` or installed row is selectable".to_owned()
        }
    });
    parts.join(" · ")
}
