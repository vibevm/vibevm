//! One explicit offline producer over the existing verified-copy/bootstrap owners.
use super::*;

pub(super) fn run(ctx: &output::Context, file: &Path, out: &Path) -> Result<()> {
    let (source, name) = recipe_scope(file)?;
    let bytes = source
        .read_file_snapshot_bounded(&name, RECIPE_LIMIT)
        .map_err(|_| anyhow!("kit recipe unavailable or unsafe"))?
        .ok_or_else(|| anyhow!("kit recipe missing"))?;
    let recipe: Recipe = toml::from_str(
        std::str::from_utf8(&bytes.bytes).map_err(|_| anyhow!("kit recipe must be UTF-8"))?,
    )
    .map_err(|_| anyhow!("invalid kit recipe TOML or unknown fields; raw input omitted"))?;
    recipe.validate_shape()?;
    let (destination, output_name) = recipe_scope(out)?;
    let output_path = destination.root_path().join(&output_name);
    ensure!(
        !output_path.starts_with(source.root_path())
            && !source.root_path().starts_with(&output_path),
        "kit output must not overlap recipe/input scope"
    );
    let parent = destination.root_dir()?;
    ensure!(
        parent
            .open_child_checked(&output_name)
            .map_err(|_| anyhow!("kit output occupied or unsafe"))?
            .is_none(),
        "kit output already exists; replacement refused"
    );
    ensure!(
        cfg!(target_os = "linux"),
        "kit publication first slice requires Linux"
    );
    parent
        .verify_user_private()
        .map_err(|_| anyhow!("kit output parent must be current-user owned with mode 0700"))?;
    // Kit build authorizes publication of its own private exclusive stage.
    // The existing cooperative Linux move grants no retirement/cleanup authority.
    let token = format!(
        "kit-build-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let stage_name = format!(".{token}");
    let stage_root = parent
        .create_private_child_exclusive(&stage_name)
        .map_err(|_| anyhow!("kit private stage could not be created exclusively"))?;
    let stage = Project::open(stage_root.path()).map_err(|_| anyhow!("kit stage unavailable"))?;
    ensure!(
        stage.root_identity()? == stage_root.identity()?,
        "kit stage identity changed"
    );
    let stage_state = parent
        .inspect_child_state(&stage_name)?
        .ok_or_else(|| anyhow!("kit stage name disappeared"))?;
    let bundle_name = format!("vibevm-{}-{}.zip", recipe.kit.version, recipe.kit.target);
    let installer_name = format!("vibe-installer-{}", recipe.kit.target);
    let profile_name = "profile-linux-auto.toml";
    let mut sums = String::new();
    for (input, target, mode) in [
        (&recipe.bundle, bundle_name.as_str(), Some(0o644)),
        (&recipe.installer, installer_name.as_str(), Some(0o755)),
        (&recipe.site_profile, profile_name, Some(0o644)),
    ] {
        let state = source
            .stable_file_state(&input.path)
            .map_err(|_| anyhow!("kit source input unsafe"))?
            .ok_or_else(|| anyhow!("kit source input missing"))?;
        ensure!(
            state.sha256 == input.sha256,
            "kit source input hash mismatch"
        );
        source
            .copy_stable_file_to_expected(
                &input.path,
                &stage,
                target,
                mode,
                &input.sha256,
                state.bytes,
            )
            .map_err(|_| anyhow!("kit verified stage copy failed; output not published"))?;
        let staged = stage
            .stable_file_state(target)?
            .ok_or_else(|| anyhow!("kit staged input missing"))?;
        ensure!(
            staged.sha256 == input.sha256 && staged.bytes == state.bytes,
            "kit staged input differs from recipe"
        );
        sums.push_str(&format!("{}  {target}\n", staged.sha256));
    }
    let verified = vibe_bootstrap::verify_bundle(
        &stage_root.path().join(&bundle_name),
        &recipe.bundle.sha256,
        &recipe.kit.target,
    )
    .map_err(|_| anyhow!("kit staged bundle invalid; output not published"))?;
    ensure!(
        verified.manifest.version == recipe.kit.version
            && verified.manifest.target == recipe.kit.target,
        "kit bundle version or target differs from recipe"
    );
    let profile = stage
        .read_file_snapshot_bounded(profile_name, PROFILE_LIMIT)?
        .ok_or_else(|| anyhow!("kit staged profile missing"))?;
    ensure!(
        profile.sha256 == recipe.site_profile.sha256,
        "kit staged profile changed"
    );
    vibe_core::site_profile::SiteProfile::parse(
        std::str::from_utf8(&profile.bytes).map_err(|_| anyhow!("kit profile must be UTF-8"))?,
        "kit staged profile",
    )
    .map_err(|_| anyhow!("kit staged profile invalid; contents omitted"))?;
    stage.publish_new_in(&stage_root, "SHA256SUMS.txt", sums.as_bytes())?;
    stage.publish_new_in(&stage_root, "INSTALL.md", b"# Local kit\n\nRun the named native installer with --kit pointing to this directory.\nUse --dry-run first; --no-path preserves user PATH.\nThe installer is checksum-pinned by the explicit recipe, not separately authenticated.\n")?;
    let names = [
        &bundle_name,
        &installer_name,
        profile_name,
        "SHA256SUMS.txt",
        "INSTALL.md",
    ];
    let expected = observe(&stage, &stage_root, &names)?;
    parent.verify_user_private()?;
    parent.retire_directory_noreplace_to(&parent, &stage_name, &output_name, &stage_state)
        .map_err(|_| anyhow!("kit publication refused or uncertain; preserve destination and private stage for inspection"))?;
    let published = parent
        .open_child_checked(&output_name)?
        .ok_or_else(|| anyhow!("kit published directory missing"))?;
    ensure!(
        published.identity()? == stage_root.identity()?,
        "kit published identity changed"
    );
    let final_project = Project::open(published.path())?;
    ensure!(
        final_project.root_identity()? == published.identity()?,
        "kit published root changed"
    );
    ensure!(
        observe(&final_project, &published, &names)? == expected,
        "kit published bytes or membership changed; preserve output for inspection"
    );
    ctx.summary("Verified local kit produced in new output directory; no installation or external publication performed.");
    Ok(())
}

fn observe(
    project: &Project,
    root: &Pinned,
    names: &[&str],
) -> Result<Vec<vibe_safefs::StableFileState>> {
    ensure!(
        root.private_entries_are_only(names)?,
        "kit stage contains unexpected names"
    );
    names
        .iter()
        .map(|name| {
            project
                .stable_file_state(name)?
                .ok_or_else(|| anyhow!("kit expected member missing"))
        })
        .collect()
}
