//! One explicit offline producer over the existing verified-copy/bootstrap owners.
use super::*;

pub(super) fn run(ctx: &output::Context, file: &Path, out: &Path) -> Result<()> {
    let (source, name) = recipe_scope(file)?;
    let path = source.root_path().join(&name);
    let bytes = source
        .read_file_snapshot_bounded(&name, RECIPE_LIMIT)
        .map_err(|_| {
            refuse(
                "kit_recipe_unreadable",
                format!("kit recipe {} cannot be read safely", path.display()),
                None,
                None,
            )
        })?
        .ok_or_else(|| {
            refuse(
                "kit_recipe_missing",
                format!("kit recipe {} does not exist", path.display()),
                Some(&format!("`vibe kit init --file {}`", refusal::quote(&path))),
                None,
            )
        })?;
    let recipe: Recipe = toml::from_str(std::str::from_utf8(&bytes.bytes).map_err(|_| {
        refuse(
            "kit_recipe_invalid",
            format!("kit recipe {} is not UTF-8", path.display()),
            None,
            None,
        )
    })?)
    .map_err(|_| {
        refuse(
            "kit_recipe_invalid",
            format!("kit recipe {} is not a valid recipe", path.display()),
            None,
            Some("invalid TOML or an unknown field; its contents are not shown"),
        )
    })?;
    recipe.validate_shape()?;
    let (destination, output_name) = recipe_scope(out)?;
    let output_path = destination.root_path().join(&output_name);
    if !(!output_path.starts_with(source.root_path())
        && !source.root_path().starts_with(&output_path))
    {
        return Err(refuse(
            "kit_output_unsafe",
            format!(
                "kit output {} overlaps the recipe directory {}",
                output_path.display(),
                source.root_path().display()
            ),
            Some("choose an output outside the recipe directory"),
            None,
        ));
    }
    let parent = destination.root_dir()?;
    if !(parent
        .open_child_checked(&output_name)
        .map_err(|_| {
            refuse(
                "kit_output_unsafe",
                format!(
                    "kit output {} cannot be inspected safely",
                    output_path.display()
                ),
                None,
                None,
            )
        })?
        .is_none())
    {
        return Err(refuse(
            "kit_output_exists",
            format!("kit output {} already exists", output_path.display()),
            Some("choose a new --out directory; kit never replaces one"),
            None,
        ));
    }
    if !(cfg!(target_os = "linux")) {
        return Err(refuse(
            "kit_unsupported_platform",
            "kit build runs only on Linux in this release",
            None,
            None,
        ));
    }
    parent
        .verify_user_private()
        .map_err(|error| refusal::output_parent(destination.root_path(), &error))?;
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
        .map_err(|_| {
            refuse(
                "kit_build_failed",
                "kit private stage could not be created exclusively",
                None,
                None,
            )
        })?;
    let stage = Project::open(stage_root.path())
        .map_err(|_| refuse("kit_build_failed", "kit stage unavailable", None, None))?;
    if !(stage.root_identity()? == stage_root.identity()?) {
        return Err(refuse(
            "kit_build_failed",
            "kit stage identity changed",
            None,
            None,
        ));
    }
    let stage_state = parent
        .inspect_child_state(&stage_name)?
        .ok_or_else(|| refuse("kit_build_failed", "kit stage name disappeared", None, None))?;
    let bundle_name = format!("vibevm-{}-{}.zip", recipe.kit.version, recipe.kit.target);
    let (installer_name, platform) = vibe_bootstrap::kit::kit_layout(&recipe.kit.target)?;
    let profile_name = format!("profile-{platform}-auto.toml");
    let profile_name = profile_name.as_str();
    let mut sums = String::new();
    for (input, target, mode, label) in [
        (&recipe.bundle, bundle_name.as_str(), Some(0o644), "bundle"),
        (
            &recipe.installer,
            installer_name.as_str(),
            Some(0o755),
            "installer",
        ),
        (
            &recipe.site_profile,
            profile_name,
            Some(0o644),
            "site profile",
        ),
    ] {
        let state = source
            .stable_file_state(&input.path)
            .map_err(|_| {
                refuse(
                    "kit_input_unsafe",
                    format!(
                        "the {label} input {} is not safe, or changed while it was read",
                        input.path
                    ),
                    None,
                    None,
                )
            })?
            .ok_or_else(|| refusal::missing(&source, &input.path, label))?;
        if state.sha256 != input.sha256 {
            return Err(refuse(
                "kit_input_mismatch",
                format!(
                    "the {label} input {} does not match its sha256 in the recipe",
                    input.path
                ),
                Some("put the intended file there, or record its sha256 in the recipe"),
                None,
            ));
        }
        source
            .copy_stable_file_to_expected(
                &input.path,
                &stage,
                target,
                mode,
                &input.sha256,
                state.bytes,
            )
            .map_err(|_| {
                refuse(
                    "kit_build_failed",
                    "kit verified stage copy failed; output not published",
                    None,
                    None,
                )
            })?;
        let staged = stage
            .stable_file_state(target)?
            .ok_or_else(|| refuse("kit_build_failed", "kit staged input missing", None, None))?;
        if !(staged.sha256 == input.sha256 && staged.bytes == state.bytes) {
            return Err(refuse(
                "kit_build_failed",
                "kit staged input differs from recipe",
                None,
                None,
            ));
        }
        sums.push_str(&format!("{}  {target}\n", staged.sha256));
    }
    let verified = vibe_bootstrap::verify_bundle(
        &stage_root.path().join(&bundle_name),
        &recipe.bundle.sha256,
        &recipe.kit.target,
    )
    .map_err(|_| {
        refuse(
            "kit_build_failed",
            "kit staged bundle invalid; output not published",
            None,
            None,
        )
    })?;
    if !(verified.manifest.version == recipe.kit.version
        && verified.manifest.target == recipe.kit.target)
    {
        return Err(refuse(
            "kit_build_failed",
            "kit bundle version or target differs from recipe",
            None,
            None,
        ));
    }
    let profile = stage
        .read_file_snapshot_bounded(profile_name, PROFILE_LIMIT)?
        .ok_or_else(|| refuse("kit_build_failed", "kit staged profile missing", None, None))?;
    if profile.sha256 != recipe.site_profile.sha256 {
        return Err(refuse(
            "kit_build_failed",
            "kit staged profile changed",
            None,
            None,
        ));
    }
    vibe_core::site_profile::SiteProfile::parse(
        std::str::from_utf8(&profile.bytes)
            .map_err(|_| refuse("kit_build_failed", "kit profile must be UTF-8", None, None))?,
        "kit staged profile",
    )
    .map_err(|_| {
        refuse(
            "kit_build_failed",
            "kit staged profile invalid; contents omitted",
            None,
            None,
        )
    })?;
    stage.publish_new_in(&stage_root, "SHA256SUMS.txt", sums.as_bytes())?;
    stage.publish_new_in(&stage_root, "INSTALL.md", b"# Local kit\n\nRun the named native installer with --kit pointing to this directory.\nUse --dry-run first; --no-path preserves user PATH.\nFor an existing installation, explicitly request --upgrade --destination PATH --no-path.\nUpgrade dry-run checks the exact active instance and requires a strictly newer verified release.\nOrdinary rerun is not an upgrade; unattended alone grants no replacement consent.\nThe installer is checksum-pinned by the explicit recipe, not separately authenticated.\n")?;
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
        .map_err(|_| refuse("kit_build_failed", "kit publication refused or uncertain; preserve destination and private stage for inspection", None, None))?;
    let published = parent.open_child_checked(&output_name)?.ok_or_else(|| {
        refuse(
            "kit_build_failed",
            "kit published directory missing",
            None,
            None,
        )
    })?;
    if !(published.identity()? == stage_root.identity()?) {
        return Err(refuse(
            "kit_build_failed",
            "kit published identity changed",
            None,
            None,
        ));
    }
    let final_project = Project::open(published.path())?;
    if !(final_project.root_identity()? == published.identity()?) {
        return Err(refuse(
            "kit_build_failed",
            "kit published root changed",
            None,
            None,
        ));
    }
    if !(observe(&final_project, &published, &names)? == expected) {
        return Err(refuse(
            "kit_build_failed",
            "kit published bytes or membership changed; preserve output for inspection",
            None,
            None,
        ));
    }
    ctx.summary("Verified local kit produced in new output directory; no installation or external publication performed.");
    Ok(())
}

fn observe(
    project: &Project,
    root: &Pinned,
    names: &[&str],
) -> Result<Vec<vibe_safefs::StableFileState>> {
    if !(root.private_entries_are_only(names)?) {
        return Err(refuse(
            "kit_build_failed",
            "kit stage contains unexpected names",
            None,
            None,
        ));
    }
    names
        .iter()
        .map(|name| {
            project.stable_file_state(name)?.ok_or_else(|| {
                refuse(
                    "kit_build_failed",
                    "kit expected member missing",
                    None,
                    None,
                )
            })
        })
        .collect()
}
