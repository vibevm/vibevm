//! `vibe doc build-site` — the registry builder's surface
//! (PROP-057 `##SITE-TWO-SOURCES`, campaign atoms A5.1–A5.3).
//!
//! The thin half, like every other `vibe doc` verb: it turns flags into
//! the library's options, hands down the ambient values the composition
//! root resolved, and prints the report. Everything that decides
//! anything lives in `vibe_doc::site`.
//!
//! The first thing a run prints is the configuration as it was RESOLVED —
//! every value, including the ones nobody wrote down. That is deliberate
//! and it is the report's main job: the configuration is read by a
//! generated type, which is permissive like every generated reader in
//! this tree, so a misspelled `[souce.host]` is not a refusal but a host
//! that is not there. Saying which sources were understood, at the top of
//! the run, catches that in the place an operator is already looking.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-TWO-SOURCES");

pub mod prepare;
pub mod web;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use vibe_doc::citations::SpecSources;
use vibe_doc::site::{Site, feed, level0, queue, render, state};
use vibe_wire::generated::doc_site_state::RenderedVersion;

use super::{DocEnv, observe};
use crate::cli::DocBuildSiteArgs;

/// Where the rendered trees are kept inside the output.
const TREES_DIR: &str = "trees";

/// Run `vibe doc build-site`.
pub fn run(args: DocBuildSiteArgs, env: DocEnv) -> Result<()> {
    let site = observe::phase(
        &env.progress,
        "Reading documentation site configuration",
        || Site::read(&args.config),
    )?;
    print!("{}", site.render());

    // The clock is called HERE and nowhere below, like every other `vibe
    // doc` verb: the debounce is arithmetic over an instant the caller
    // supplies, and a manifest carries one, so a build can be replayed
    // and reviewed.
    let now = Utc::now();
    let polled = observe::phase(&env.progress, "Polling documentation sources", || {
        feed::poll(&site)
    })?;
    for line in &polled.sources {
        println!("  {line}");
    }

    // A featured coordinate nothing publishes is said out loud and does
    // not stop the render: the registry moves without this file, so a
    // package renamed yesterday must not cost the domain today's build.
    // Silence would leave the front of the site quietly featuring one
    // documentation fewer than the configuration names.
    let catalogue: std::collections::BTreeSet<String> =
        polled.pairs.iter().map(|pair| pair.coordinate()).collect();
    for coordinate in site.featured_absent(&catalogue) {
        println!(
            "  warn   featured {coordinate} — no source publishes it, so the front of the \
             site shows one documentation fewer"
        );
    }

    let mut rendered = observe::phase(&env.progress, "Reading documentation site state", || {
        state::read(&args.out)
    })?;
    let debounce = site
        .host
        .as_ref()
        .map(|host| host.debounce_minutes)
        .unwrap_or_default();
    let plan = queue::plan(&polled.pairs, &rendered, now, debounce);
    print!("{}", plan.render());

    if args.dry_run {
        println!("  dry run — nothing was written");
        return Ok(());
    }

    let work = args.out.join(state::STATE_DIR).join(TREES_DIR);
    let sources = spec_sources(&site, &env);
    let builder = observe::phase(
        &env.progress,
        "Preparing documentation site builder",
        || {
            composition_root(
                &site,
                &env,
                plan.rebuild.iter().any(|queued| !host(&queued.pair)),
            )
        },
    )?;

    // The reverse edges, folded ONCE over the whole catalog and then
    // read per package (`##REL-REVERSE-QUERIES-SITE-SIDE`). Folded from
    // the FEED and not from what is rendered, so a package's shelf shows
    // what the registry holds now rather than what this run happened to
    // rebuild.
    let shelves = vibe_doc::site::shelves::fold(&facts(&polled.pairs));

    let render_versions = env.progress.task("Rendering documentation versions");
    render_versions.set_progress(0, Some(plan.rebuild.len() as u64), "versions");
    let mut failures = 0;
    let mut render_output = Vec::new();
    for (index, queued) in plan.rebuild.iter().enumerate() {
        let item = render_versions
            .progress()
            .task(format!("Rendering {}", queued.pair.spelled()));
        let related = level0::Related::of(&shelves, &queued.pair.coordinate());
        let options = render::Options {
            base: &site.base,
            sources: &sources,
            work: &work,
            rendered_at: now,
            related: &related,
        };
        let out = match render::render(&queued.pair, &builder, &options) {
            Ok(out) => out,
            Err(error) => {
                item.fail("version render failed");
                render_versions.fail("documentation site render failed");
                for line in render_output {
                    println!("{line}");
                }
                return Err(error.into());
            }
        };
        for note in &out.notes {
            render_output.push(format!("  note   {} — {note}", queued.pair.spelled()));
        }
        if let Some(reason) = &out.failed {
            failures += 1;
            render_output.push(format!("  failed {} — {reason}", queued.pair.spelled()));
        }
        if let Err(error) = record(&mut rendered, &queued.pair, &out, now) {
            item.fail("version state recording failed");
            render_versions.fail("documentation site state recording failed");
            for line in render_output {
                println!("{line}");
            }
            return Err(error);
        }
        if out.failed.is_some() {
            item.fail("version render refused");
        } else {
            item.finish();
        }
        render_versions.set_progress(
            (index + 1) as u64,
            Some(plan.rebuild.len() as u64),
            "versions",
        );
    }
    render_versions.finish();
    for line in render_output {
        println!("{line}");
    }

    // An address no source publishes any more stops being served. The
    // registry keeps no history, so a version that left the catalog left
    // it, and a page that outlived its package would be the only place
    // it still existed.
    let remove_versions = env
        .progress
        .task("Removing unpublished documentation versions");
    remove_versions.set_progress(0, Some(plan.gone.len() as u64), "versions");
    let mut removed_output = Vec::new();
    for (index, address) in plan.gone.iter().enumerate() {
        match forget(&mut rendered, address, &work) {
            Ok(Some(line)) => removed_output.push(line),
            Ok(None) => {}
            Err(error) => {
                remove_versions.fail("documentation version removal failed");
                for line in removed_output {
                    println!("{line}");
                }
                return Err(error);
            }
        }
        remove_versions.set_progress((index + 1) as u64, Some(plan.gone.len() as u64), "versions");
    }
    remove_versions.finish();
    for line in removed_output {
        println!("{line}");
    }
    // The debounce runs from the moment the host's half completed, not
    // from the moment anybody last asked: a run that rendered no host
    // pair must not restart the clock, or a branch polled every minute
    // would never be rendered at all.
    if plan.rebuild.iter().any(|queued| host(&queued.pair)) {
        rendered.host_rendered_at = Some(now);
    }
    observe::phase(&env.progress, "Writing documentation site state", || {
        state::write(&args.out, &rendered)
    })?;
    println!(
        "render: {} version(s) written, {failures} refused, {} standing",
        plan.rebuild.len(),
        rendered.rendered.len(),
    );

    // The address map, recomputed from what STANDS rather than from what
    // moved: `latest` is an alias over everything the sources publish
    // now, and a build that computed it from this run's queue would name
    // whichever version happened to be rebuilt.
    print!(
        "{}",
        vibe_doc::site::addresses::of(&published(&rendered, &work)).render()
    );

    let trees = every_tree(&rendered, &work);
    match web_package(&args, &site) {
        Some(web) => {
            println!(
                "  site   {} tree(s) handed to {}",
                trees.len(),
                web.display()
            );
            let report = observe::phase(&env.progress, "Building documentation web shell", || {
                web::build(&web, &trees, &site, &args.out)
            })?;
            print!("{report}");
        }
        None if args.no_web => println!("  site   not built — `--no-web`"),
        None => println!(
            "  site   not built — the site package is not beside the host's checkout; \
             pass `--web <dir>`"
        ),
    }
    println!("  output {}", args.out.display());
    Ok(())
}

/// Is this pair the host's?
fn host(pair: &vibe_doc::site::Pair) -> bool {
    !matches!(pair.origin, vibe_doc::site::Origin::Registry)
}

/// What every polled version states, in the one shape the reverse fold
/// reads.
///
/// A registry pair carries its catalog record and is read from it; a
/// host pair has none — its coordinate is in no index — and is read from
/// the manifest the record would have been made from. Two sources, one
/// vocabulary: without the second, the host's own documentation would be
/// invisible on the host's page, which is the one package this project
/// is certain to want a shelf for.
fn facts(pairs: &[vibe_doc::site::Pair]) -> Vec<vibe_doc::site::shelves::Facts> {
    let mut out = Vec::new();
    for pair in pairs {
        if let Some(entry) = &pair.entry {
            out.push(vibe_doc::site::shelves::from_entry(entry));
            continue;
        }
        let dir = match &pair.origin {
            vibe_doc::site::Origin::HostProject { root } => root,
            vibe_doc::site::Origin::HostPackage { dir } => dir,
            vibe_doc::site::Origin::Registry => continue,
        };
        if let Some(facts) = vibe_doc::site::shelves::from_manifest(dir) {
            out.push(facts);
        }
    }
    out
}

/// Record what one render produced, replacing the row it had.
fn record(
    state: &mut vibe_wire::generated::doc_site_state::DocSiteState,
    pair: &vibe_doc::site::Pair,
    out: &render::Rendered,
    now: chrono::DateTime<Utc>,
) -> Result<()> {
    let row = RenderedVersion {
        source: pair.source.clone(),
        group: vibe_core::Group::parse(&pair.group)
            .with_context(|| format!("`{}` is not a group", pair.group))?,
        name: pair.name.clone(),
        version: pair
            .version
            .parse()
            .with_context(|| format!("`{}` is not a version", pair.version))?,
        content_hash: pair.content_hash.clone(),
        rendered_at: now,
        files: u32::try_from(out.files).unwrap_or(u32::MAX),
        failed: !out.ok(),
    };
    state
        .rendered
        .retain(|standing| state::key(standing) != state::key(&row));
    state.rendered.push(row);
    Ok(())
}

/// Drop an address the sources no longer publish, and the trees behind
/// it.
fn forget(
    state: &mut vibe_wire::generated::doc_site_state::DocSiteState,
    address: &str,
    work: &Path,
) -> Result<Option<String>> {
    let Some((coordinate, version)) = address.rsplit_once('@') else {
        return Ok(None);
    };
    let Some((group, name)) = coordinate.rsplit_once('/') else {
        return Ok(None);
    };
    let slot = work.join(format!("{group}.{name}@{version}"));
    if slot.is_dir() {
        std::fs::remove_dir_all(&slot).with_context(|| format!("removing `{}`", slot.display()))?;
    }
    state.rendered.retain(|row| {
        !(row.group.to_string() == group && row.name == name && row.version.to_string() == version)
    });
    Ok(Some(format!(
        "  gone   {address} — its pages are no longer published"
    )))
}

/// Every version the output stands on, as the address map sees it.
///
/// The language and the page count are read from the manifest the build
/// WROTE rather than remembered from the run, so a version rendered an
/// hour ago and one rendered a second ago answer the same way. A tree
/// whose manifest cannot be read is left out: the address map would
/// otherwise count pages it cannot name.
fn published(
    state: &vibe_wire::generated::doc_site_state::DocSiteState,
    work: &Path,
) -> Vec<vibe_doc::site::Published> {
    let mut out = Vec::new();
    for row in &state.rendered {
        let manifest = work
            .join(format!("{}.{}@{}", row.group, row.name, row.version))
            .join(render::FORMATS[0].as_str())
            .join("manifest.json");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        let Ok(read) =
            serde_json::from_str::<vibe_wire::generated::doc_manifest::DocManifest>(&text)
        else {
            continue;
        };
        out.push(vibe_doc::site::Published {
            coordinate: format!("{}/{}", row.group, row.name),
            version: row.version.to_string(),
            lang: read.package.lang,
            pages: read.pages.len(),
        });
    }
    out
}

/// Every tree the static build is handed: one per projection of every
/// version that stands, rendered in this run or an earlier one.
///
/// All of them, and not only what moved. The site is built whole from
/// the trees each time — a page's neighbours, the language selector and
/// the catalogue are folded out of the set — so handing over only the
/// rebuilt ones would publish a site of whatever changed this hour.
fn every_tree(
    state: &vibe_wire::generated::doc_site_state::DocSiteState,
    work: &Path,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for row in &state.rendered {
        for format in render::FORMATS {
            let tree = work
                .join(format!("{}.{}@{}", row.group, row.name, row.version))
                .join(format.as_str());
            if tree.is_dir() {
                out.push(tree);
            }
        }
    }
    out
}

/// Which site package builds the domain.
fn web_package(args: &DocBuildSiteArgs, site: &Site) -> Option<PathBuf> {
    if args.no_web {
        return None;
    }
    args.web.clone().or_else(|| web::beside(site))
}

/// The world a `rule` citation resolves against for a site build: the
/// machine store the warm-up fills, and the host's checkout when this
/// build carries one.
///
/// Named here rather than discovered, for the reason every other `vibe
/// doc` verb names it: a documentation build that depended on an
/// undeclared search path could not be reproduced by reading it.
fn spec_sources(site: &Site, env: &DocEnv) -> SpecSources {
    let sources = match site.host.as_ref() {
        Some(host) => {
            let (group, name) = super::self_coordinate(&host.checkout);
            SpecSources::for_checkout(&host.checkout, group.as_deref(), &name)
        }
        None => SpecSources::new(),
    };
    match super::settings_home(&env.settings, &env.home) {
        Some(home) => sources.with_store(home.join(super::STORE_DIR)),
        None => sources,
    }
}

/// The composition root of this build: the registries this machine
/// reads, its store, and the product whose `derived` blocks the host's
/// own documentation shows.
fn composition_root(site: &Site, env: &DocEnv, warms: bool) -> Result<prepare::Builder> {
    let root = site
        .host
        .as_ref()
        .map(|host| host.checkout.clone())
        .or_else(|| env.cwd.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    // The resolver is opened only when something has to be warmed. A
    // site whose only source is the host's checkout reads no registry at
    // all, and opening one would refuse a buildable site over a
    // `[[registry]]` nobody needed.
    let resolver = match warms {
        true => Some(crate::commands::cache::add::cache_resolver(&root, false)?.0),
        false => None,
    };
    Ok(prepare::Builder {
        resolver,
        store_root: vibe_registry::store_root().context("resolving the machine store root")?,
        host: site.host.as_ref().and_then(|host| {
            env.current_exe.clone().map(|binary| prepare::HostProduct {
                binary,
                checkout: host.checkout.clone(),
            })
        }),
        timeout_secs: DERIVED_TIMEOUT_SECS,
    })
}

/// Seconds one generated block may take. The same ceiling `vibe doc
/// build` uses, because it is the same generator.
const DERIVED_TIMEOUT_SECS: u64 = 300;

#[cfg(test)]
mod tests;
