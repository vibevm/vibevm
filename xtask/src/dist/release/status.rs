use super::*;

pub(crate) fn status(repo_root: &Path, raw_version: &str) -> Result<()> {
    let identity = committed_identity(repo_root, false)?;
    let version = checked_version(repo_root, raw_version, &identity)?;
    let client = write_client()?;
    let tag = format!("v{version}");
    let release = client
        .find_release_authenticated(&tag)?
        .with_context(|| format!("GitHub release `{tag}` does not exist; run `dist prepare`"))?;
    let assets = unique_assets(client.list_assets_authenticated(release.id)?)?;
    println!(
        "dist status: {} ({})",
        tag,
        if release.draft { "draft" } else { "published" }
    );
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let expected = [
            bundle_asset_name(&version, target),
            bootstrap_asset_name(target),
            fragment_asset_name(&version, target),
        ];
        let present = expected
            .iter()
            .filter(|name| assets.contains_key(*name))
            .count();
        println!("  {target}: {present}/3 assets");
    }
    for optional in [
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
        DISTRIBUTION_BASH_INSTALLER_FILENAME,
        DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
    ] {
        println!(
            "  {optional}: {}",
            if assets.contains_key(optional) {
                "present"
            } else {
                "absent"
            }
        );
    }
    Ok(())
}
