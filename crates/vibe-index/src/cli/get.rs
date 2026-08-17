//! `vibe-index get <data-dir> <group> <name>` — read one package entry
//! from the index by its `(group, name)` identity (PROP-008 §2.2).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::index::quarantine::{
    Unavailable, unavailable_for, usable_latest_stable, usable_versions,
};
use crate::types::{PackageEntry, VersionEntry};

#[derive(Debug, Parser)]
#[command(about = "Read one package entry from the index.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Reverse-FQDN group qualifier — e.g. `org.vibevm`.
    pub group: Group,

    pub name: String,

    /// Specific version. If omitted, prints every version.
    #[arg(long, value_name = "SEMVER")]
    pub version: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct GetEnvelope<'a> {
    command: &'static str,
    found: bool,
    group: &'a Group,
    name: &'a str,
    versions: Vec<&'a VersionEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    unavailable: Vec<Unavailable>,
}

pub fn run(args: Args) -> Result<()> {
    let index = Index::load_from(&args.data_dir)?;
    let Some(pkg) = index.get(&args.group, &args.name) else {
        if args.json {
            let env = GetEnvelope {
                command: "get",
                found: false,
                group: &args.group,
                name: &args.name,
                versions: vec![],
                unavailable: vec![],
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&env)
                    .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
            );
            return Ok(());
        }
        return Err(Error::InvalidInput(format!(
            "package `{}/{}` is not in the index",
            args.group, args.name
        )));
    };

    let req: Option<semver::Version> = match &args.version {
        Some(v) => Some(v.parse().map_err(|e| {
            Error::InvalidInput(format!("`--version {v}` is not valid semver: {e}"))
        })?),
        None => None,
    };
    let versions: Vec<&VersionEntry> = match &req {
        Some(req) => usable_versions(pkg)
            .filter(|ve| ve.version == *req)
            .collect(),
        None => usable_versions(pkg).collect(),
    };
    // The refusal rows: every version this build cannot act on, narrowed
    // to the ask when the ask named one version.
    let mut unavailable = unavailable_for(pkg);
    if let Some(req) = &req {
        unavailable.retain(|u| &u.version == req);
    }

    if versions.is_empty() && unavailable.is_empty() {
        // Nothing usable and nothing refused: the ask named a version
        // this index does not hold at all (or the package row carries
        // no versions). The honest `found:false` / error — no version
        // exists to speak about, so `unavailable` stays silent too.
        if args.json {
            let env = GetEnvelope {
                command: "get",
                found: false,
                group: &args.group,
                name: &args.name,
                versions,
                unavailable,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&env)
                    .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
            );
            return Ok(());
        }
        return Err(Error::InvalidInput(format!(
            "package `{}/{}` has no version `{}` in the index",
            args.group,
            args.name,
            req.as_ref().map(|v| v.to_string()).unwrap_or_default()
        )));
    }

    // `found` keeps its meaning — the `(group, name)` identity STANDS
    // in the index: `true` for a whole-package ask even when every
    // version is refused (the refusal is named beside it, not hidden
    // behind a `false`). A specific `--version` ask stays `false` when
    // that version was not SERVED — the asked-for fact did not come
    // back, `unavailable` says why.
    let found = req.is_none() || !versions.is_empty();
    if args.json {
        let env = GetEnvelope {
            command: "get",
            found,
            group: &args.group,
            name: &args.name,
            versions,
            unavailable,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&env)
                .map_err(|e| Error::Malformed(format!("envelope: {e}")))?
        );
    } else {
        render_text(pkg, &versions, &unavailable);
    }
    Ok(())
}

fn render_text(pkg: &PackageEntry, versions: &[&VersionEntry], unavailable: &[Unavailable]) {
    println!("group         : {}", pkg.group);
    println!("name          : {}", pkg.name);
    if let Some(kind) = usable_versions(pkg).next().map(|v| &v.kind) {
        println!("kind          : {kind}");
    }
    if let Some(latest) = usable_latest_stable(pkg) {
        println!("latest stable : {latest}");
    }
    println!("versions      : {}", versions.len());
    for v in versions {
        println!(
            "  - {} (commit {})",
            v.version,
            v.resolved_commit.as_deref().unwrap_or("-")
        );
        if let Some(d) = &v.description {
            println!("    {d}");
        }
        println!("    content_hash: {}", v.content_hash);
        println!("    source_url  : {}", v.source_url);
    }
    if !unavailable.is_empty() {
        println!("unavailable   : {}", unavailable.len());
        for u in unavailable {
            println!("  - {}  missing: {}", u.version, u.missing.join(","));
            println!("    {}", u.recipe);
        }
    }
}
