//! Read-only request-local data from a verified local owned offer, not supplier provenance.
use super::home::RemapHome;
use crate::{cli::RemapPreviewArgs, output};
use anyhow::{Result, anyhow, ensure};
use vibe_core::manifest::Manifest;
pub(super) fn run(ctx: &output::Context, args: RemapPreviewArgs, offline: bool) -> Result<()> {
    ensure!(offline, "remap preview requires explicit offline operation");
    let home = RemapHome::open()?;
    let key = args.package.strip_prefix("remap:").unwrap_or(&args.package);
    let reference = vibe_core::PackageRef::parse(key)?;
    ensure!(
        reference.group.is_some() && reference.qualified_name() == key,
        "remap preview requires one explicit unversioned qualified offer key"
    );
    let entries: Vec<_> = home
        .installed
        .packages
        .iter()
        .filter(|e| e.package.key == key)
        .collect();
    ensure!(
        entries.len() == 1,
        "remap preview requires exactly one local offer"
    );
    let entry = entries[0];
    let scope = vibe_safefs::Project::open(home.root())?;
    let index = scope
        .read_file_snapshot_bounded("remap/installed.json", 1024 * 1024)?
        .ok_or_else(|| anyhow!("local offer index missing"))?;
    let captured_index: vibe_core::remap::InstalledRemaps =
        serde_json::from_slice(&index.bytes).map_err(|_| anyhow!("local offer index invalid"))?;
    ensure!(
        captured_index == home.installed,
        "local offer index changed during selection"
    );
    let tree = super::packages::tree_of(&home, key);
    let proof =
        vibe_registry::local_preview::verify_pinned_payload(&tree, &entry.package.content_hash)?;
    let manifest = Manifest::parse_str(std::str::from_utf8(proof.manifest_bytes()?)?)?;
    ensure!(
        manifest.require_package()?.kind == vibe_core::PackageKind::Remap,
        "local offer kind mismatch"
    );
    let observed = vibe_core::remap::offer_from_manifest(&manifest, &entry.package.content_hash)
        .map_err(|_| anyhow!("local offer manifest invalid"))?;
    ensure!(
        observed == entry.package,
        "local offer manifest/rows/policy differ from recorded offer"
    );
    let path = std::path::absolute(&args.input)?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("preview input parent missing"))?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("preview input filename invalid"))?;
    let input_scope = vibe_safefs::Project::open(parent)?;
    let bytes = input_scope
        .read_file_snapshot_bounded(name, 64 * 1024)?
        .ok_or_else(|| anyhow!("preview input unavailable"))?;
    let input = vibe_wire::behaviour::remap_preview::parse(&bytes.bytes)?;
    let selections = input
        .selections
        .into_iter()
        .map(|(id, answer)| (id, answer.members))
        .collect();
    let values = vibe_core::manifest::evaluate_request_local_data(&manifest, &selections)
        .map_err(|reason| anyhow!(reason))?;
    proof.recheck()?;
    ensure!(
        scope
            .read_file_snapshot_bounded("remap/installed.json", 1024 * 1024)?
            .as_ref()
            == Some(&index),
        "local offer index changed during preview"
    );
    use vibe_wire::generated::remap_preview_result::{RemapPreviewResult, Source, SourceKind};
    let result = RemapPreviewResult {
        protocol: "vibe-remap-preview/1".into(),
        package: key.into(),
        version: entry.package.version.clone(),
        source: Source {
            kind: SourceKind::LocalOwnedOffer,
            content_hash: entry.package.content_hash.clone(),
        },
        selected_values: values
            .into_iter()
            .map(|(id, value)| Ok((id, Some(serde_json::to_value(value)?))))
            .collect::<Result<_>>()?,
    };
    if args.json || ctx.is_json() {
        ctx.emit_json(&result)
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }
}
