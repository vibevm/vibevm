//! `vibe app versions PACKAGE|SLOT`: one listing over a package's whole
//! version domain (PROP-075 `#VERSIONS-LIST`, `#VERSIONS-LIST-OUTPUT`).
//!
//! The library in `vibe-app` joins a domain with this home's index; this
//! adapter supplies the domain and draws the rows. The domain is the
//! **skeleton** stage of PROP-075 `#TEMPLATE-TWO-STAGE`: a template's
//! version list and nothing else — no digest document, no render, no
//! installation. Inside the version-list TTL, under `--offline`, and
//! under `--installed` it costs no request at all.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT");

use std::path::{Path, PathBuf};

use anyhow::Result;
use vibe_app::{Domain, DomainVersion, Domains, Origin, SettingsRoot, TemplateFacts};
use vibe_core::Group;
use vibe_registry::templates::{ActiveTemplate, DirectoryTemplate, TemplateLayer};
use vibe_wire::generated::application_versions as wire;

use super::global;
use crate::cli::AppVersionsArgs;
use crate::output;

pub(super) fn run(
    ctx: &output::Context,
    home: &SettingsRoot,
    args: AppVersionsArgs,
    embedded_root: Option<&Path>,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<()> {
    let index = vibe_app::Index::load(home)?;
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
    };
    let request = vibe_app::VersionsRequest {
        target: &args.target,
        channel: args.channel.as_deref(),
        installed: args.installed,
        limit: args.limit,
        all: args.all,
    };
    let listing =
        ctx.suspend_progress(|| vibe_app::versions_listing(&index, &request, &domains))?;
    if ctx.is_json() {
        return ctx.emit_json(&listing);
    }
    print(&listing, ctx.is_quiet());
    Ok(())
}

/// The version domain of one package, read through the same source chain
/// an install resolves through.
struct RegistryDomains {
    settings: SettingsRoot,
    registry: Option<PathBuf>,
    embedded: Option<PathBuf>,
    offline: bool,
    operation: vibe_package_source::SourceOperation,
    cache_only: bool,
}

impl Domains for RegistryDomains {
    fn domain(&self, group: &str, name: &str) -> Result<Domain, String> {
        let group = Group::parse(group).map_err(|error| error.to_string())?;
        let Some(layer) = vibe_registry::templates::active() else {
            // No layer means no template cache and no template
            // discovery: the pre-PROP-075 answer, which is the published
            // versions alone.
            return self.published(&group, name);
        };
        if self.cache_only {
            return Ok(cached_domain(&layer, &group, name));
        }
        for root in self.roots()? {
            match layer.for_directory(&root, &group, name) {
                Ok(Some(found)) => return Ok(directory_domain(&layer, &found)),
                Ok(None) => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        self.published(&group, name)
    }
}

impl RegistryDomains {
    /// The directory-shaped registries this listing may read a template
    /// from, in precedence order (PROP-075 `#TEMPLATE-DISCOVERY`).
    fn roots(&self) -> Result<Vec<PathBuf>, String> {
        if let Some(root) = &self.registry {
            return Ok(vec![root.clone()]);
        }
        let mut roots: Vec<PathBuf> = self.embedded.clone().into_iter().collect();
        roots.extend(
            vibe_app::directory_roots_from_global_with_operation(
                self.settings.path(),
                self.offline,
                &self.operation,
            )
            .map_err(|error| error.to_string())?,
        );
        Ok(roots)
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
fn directory_domain(layer: &TemplateLayer, found: &DirectoryTemplate) -> Domain {
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
        template: Some(template_facts(layer, &found.template)),
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
        template: Some(template_facts(layer, &template)),
        notices: Vec::new(),
    }
}

fn template_facts(layer: &TemplateLayer, template: &ActiveTemplate) -> TemplateFacts {
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
        snapshot_synced: None,
    }
}

/// The human listing: one block per package, the rows newest first, and
/// the slot's trailer.
fn print(listing: &wire::ApplicationVersions, quiet: bool) {
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
    println!("{}", trailer(listing));
}

/// One row: the version, its origin, the channels it belongs to and what
/// this home holds of it.
fn row_line(row: &wire::Row) -> String {
    let mut line = format!("{} ({})", row.version, origin_label(&row.origin));
    if !row.channels.is_empty() {
        line.push_str(&format!(" [{}]", row.channels.join(", ")));
    }
    if let Some(state) = state_label(&row.state) {
        line.push_str(&format!(" — {state}"));
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

fn trailer(listing: &wire::ApplicationVersions) -> String {
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
    parts.join(" · ")
}
