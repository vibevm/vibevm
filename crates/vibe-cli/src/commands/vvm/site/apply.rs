//! `vibe self apply <file|url>`, `--dry-run`, `--diff`, `--show`,
//! `--remove` (`PROP-019#site-profile`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::remap::ByteLocation;
use vibe_core::site_profile::{
    InstalledSiteProfile, ReceiptPath, SITE_PROFILE_FILE, SITE_PROFILE_SCHEMA, SITE_RECEIPT_FILE,
    SiteProfile, SiteReceipt,
};
use vibe_core::user_config::UserConfig;
use vibe_wire::generated::self_apply_report::{
    ApplyAction, FileChange, FileChangeKind, PathReport, ProfileInfo, SelfApplyReport,
};

use super::super::VvmEnv;
use super::super::bundle::release_source;
use super::super::store::VersionStore;
use super::ConfigLayers;
use super::diff::line_diff;
use super::layers::InstallBaseOrigin;
use super::report::{self, COMMAND, PROTOCOL};
use super::steps::{import_running_if_unmanaged, path_step};
use super::view::{conflict_warnings, effective, where_to_put};
use crate::cli::VvmApplyArgs;
use crate::output;

/// A profile larger than this is not a profile.
const PROFILE_MAX_BYTES: u64 = 1024 * 1024;

/// A validated profile ready to be stored: its exact bytes and origin.
pub(in crate::commands::vvm) struct PreparedProfile {
    profile: SiteProfile,
    text: String,
    origin: String,
    sha256: String,
}

/// The two settings-directory files apply owns.
pub(super) struct Files {
    pub(super) profile: PathBuf,
    pub(super) receipt: PathBuf,
}

impl Files {
    pub(super) fn resolve() -> Result<Self> {
        let missing = || {
            anyhow!(
                "no settings directory to store the site profile in \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
                  fix: set HOME, or VIBE_SETTINGS to the settings directory)"
            )
        };
        Ok(Files {
            profile: vibe_core::settings::site_profile_path().ok_or_else(missing)?,
            receipt: vibe_core::settings::site_receipt_path().ok_or_else(missing)?,
        })
    }
}

/// `vibe self apply` (`PROP-019#site-profile`).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
pub(in crate::commands::vvm) fn run_apply_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmApplyArgs,
) -> Result<()> {
    let dry_run = args.dry_run || args.diff;
    if args.show {
        return super::manage::show(ctx, env);
    }
    if args.remove {
        return super::manage::remove(ctx, dry_run, args.diff);
    }
    let source = args.profile.as_deref().ok_or_else(|| {
        anyhow!(
            "name the site profile to apply \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
              fix: `vibe self apply <file|url>`, or `--show` / `--remove`)"
        )
    })?;
    let flag = match (args.fix_path, args.no_fix_path) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    };
    let prepared = prepare_profile(ctx, env, source)?;
    let install_root = applying_install_base(&prepared, env)?;
    let store = match &install_root {
        Some((base, _)) => VersionStore::new(base.join("opt")),
        None => env.store()?,
    };
    let mut report = install_profile(&prepared, dry_run, args.diff, install_root.clone())?;
    report.imported = import_running_if_unmanaged(ctx, dry_run, &store)?;
    // A root the environment already names, or the built-in default, needs
    // no durable write; one a file declares does, or the next terminal
    // resolves another installation.
    let durable = install_root
        .as_ref()
        .filter(|(_, origin)| origin.needs_durable_write())
        .map(|(base, _)| base.clone());
    report.path = Some(path_step(
        ctx,
        env,
        &store,
        flag,
        prepared.profile.path_fix(),
        dry_run,
        durable.as_deref(),
    )?);
    if !dry_run {
        record_path(&prepared, report.path.as_ref(), report.changed)?;
    }
    report::emit(&report, ctx)
}

/// Read and fully validate a profile from a path, a `file://` URL or an
/// `https://` / `http://` URL. Nothing is written.
pub(in crate::commands::vvm) fn prepare_profile(
    ctx: &output::Context,
    env: &VvmEnv,
    source: &str,
) -> Result<PreparedProfile> {
    let (bytes, origin) = read_source(ctx, env, source)?;
    let text = String::from_utf8(bytes).map_err(|_| {
        anyhow!(
            "`{origin}` is not UTF-8 text \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
              fix: save the profile as UTF-8 TOML)"
        )
    })?;
    let profile = SiteProfile::parse(&text, &origin)?;
    // The release source is checked here too, by its one owner, so an
    // unusable `[self]` is refused before anything is stored.
    if profile.declares_release_source() {
        release_source::resolve(None, &profile.user_config_layer(), Some(Path::new(&origin)))
            .map_err(|error| {
                anyhow!(
                    "`{origin}`: the release source (`[self]` / `[artifacts] vibe`) is unusable: {error}"
                )
            })?;
    }
    // Valid, but not usable on this host: said once, at apply and at
    // `--dry-run`, rather than discovered at the first read.
    for note in profile.windows_only_notes(cfg!(windows)) {
        eprintln!("{}", output::language::warning(&note));
    }
    let sha256 = hex(&Sha256::digest(text.as_bytes()));
    Ok(PreparedProfile {
        profile,
        text,
        origin,
        sha256,
    })
}

fn read_source(ctx: &output::Context, env: &VvmEnv, source: &str) -> Result<(Vec<u8>, String)> {
    let local = if source.contains("://") {
        match ByteLocation::parse(source)? {
            ByteLocation::Local(path) => path,
            ByteLocation::Remote(url) => {
                env.refuse_offline("self:apply", &url)?;
                return super::super::bundle::fetch_document(
                    &url,
                    PROFILE_MAX_BYTES,
                    &ctx.progress(),
                );
            }
        }
    } else {
        let path = PathBuf::from(source);
        match (&env.cwd, path.is_absolute()) {
            (Some(cwd), false) => cwd.join(path),
            _ => path,
        }
    };
    let origin = local.display().to_string();
    let metadata = std::fs::metadata(&local).with_context(|| {
        format!(
            "reading the site profile `{origin}` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
              fix: check the path)"
        )
    })?;
    if !metadata.is_file() || metadata.len() > PROFILE_MAX_BYTES {
        bail!(
            "`{origin}` is not a site profile: a regular file of at most {PROFILE_MAX_BYTES} bytes \
             is expected (violates spec://org.vibevm.core/vibevm/common/PROP-019#site-profile; \
             fix: name the profile's TOML file)"
        );
    }
    let bytes = std::fs::read(&local).with_context(|| format!("reading `{origin}`"))?;
    Ok((bytes, origin))
}

/// Store `prepared` as the site layer and write its receipt — or, for a
/// dry run, report what would change. Idempotent: the same bytes from the
/// same origin write nothing.
fn install_profile(
    prepared: &PreparedProfile,
    dry_run: bool,
    with_diff: bool,
    install_root: Option<(PathBuf, InstallBaseOrigin)>,
) -> Result<SelfApplyReport> {
    let files = Files::resolve()?;
    let stored = read_optional(&files.profile)?;
    let mut warnings = Vec::new();
    let receipt = match SiteReceipt::load_from(&files.receipt) {
        Ok(receipt) => receipt,
        Err(error) => {
            warnings.push(format!("{error}; it is rewritten"));
            None
        }
    };
    let unchanged = stored.as_deref() == Some(prepared.text.as_str())
        && receipt
            .as_ref()
            .is_some_and(|r| r.sha256 == prepared.sha256 && r.source == prepared.origin);
    let applied_at = match (&receipt, unchanged) {
        (Some(receipt), true) => receipt.applied_at.clone(),
        _ => chrono::Utc::now().to_rfc3339(),
    };
    let new_receipt = SiteReceipt {
        schema: SITE_PROFILE_SCHEMA,
        source: prepared.origin.clone(),
        sha256: prepared.sha256.clone(),
        applied_at: applied_at.clone(),
        vibe_version: env!("CARGO_PKG_VERSION").to_owned(),
        name: prepared.profile.name.clone(),
        owns: vec![SITE_PROFILE_FILE.to_owned(), SITE_RECEIPT_FILE.to_owned()],
        path: receipt.as_ref().and_then(|receipt| receipt.path.clone()),
    };
    let existing_receipt = read_optional(&files.receipt)?;
    let receipt_text = new_receipt.render(existing_receipt.as_deref(), &files.receipt)?;
    let mut changes = vec![
        change(
            &files.profile,
            stored.as_deref(),
            Some(&prepared.text),
            with_diff,
        ),
        change(
            &files.receipt,
            existing_receipt.as_deref(),
            Some(&receipt_text),
            with_diff,
        ),
    ];
    if unchanged {
        // Same bytes from the same origin: the receipt keeps its time too.
        for file in &mut changes {
            file.change = FileChangeKind::Unchanged;
            file.diff = None;
        }
    }
    let changed = changes
        .iter()
        .any(|file| file.change != FileChangeKind::Unchanged);
    if changed && !dry_run {
        write_atomic(&files.profile, prepared.text.as_bytes())?;
        write_atomic(&files.receipt, receipt_text.as_bytes())?;
    }

    let site = InstalledSiteProfile {
        path: files.profile.clone(),
        profile: prepared.profile.clone(),
    };
    let mut own_warnings = Vec::new();
    let layers = ConfigLayers {
        own: UserConfig::load_own().unwrap_or_else(|error| {
            own_warnings.push(error.to_string());
            UserConfig::default()
        }),
        own_path: UserConfig::default_path(),
        site: Some(site),
    };
    warnings.extend(own_warnings);
    let effective = effective(&layers, None, install_root, &mut warnings)?;
    let where_to_put = where_to_put(&layers, &mut warnings);
    warnings.extend(conflict_warnings(&layers, &effective));
    let mut next_steps = Vec::new();
    if !prepared.profile.registries.is_empty() {
        next_steps.push(
            "in each project resolved before this profile, move the locked sources onto the \
             site's registries: `vibe update --sources-only`"
                .to_owned(),
        );
    }
    Ok(SelfApplyReport {
        protocol: PROTOCOL.to_owned(),
        ok: true,
        command: COMMAND.to_owned(),
        action: if dry_run {
            ApplyAction::DryRun
        } else {
            ApplyAction::Apply
        },
        changed,
        profile: Some(ProfileInfo {
            name: prepared.profile.name.clone(),
            source: prepared.origin.clone(),
            sha256: prepared.sha256.clone(),
            stored: files.profile.display().to_string(),
            applied_at: (!dry_run).then_some(applied_at),
        }),
        files: changes,
        effective: Some(effective),
        where_: Some(where_to_put),
        path: None,
        imported: None,
        next_steps,
        warnings,
    })
}

/// The install root `apply` is about to use, and what decided it
/// (`PROP-019#install-root`).
///
/// The profile being applied is not stored yet, so the layers are assembled
/// from it directly: the environment variable this invocation inherited,
/// then the user's own `config.toml`, then THIS profile, then `~/.vibe`.
fn applying_install_base(
    prepared: &PreparedProfile,
    env: &VvmEnv,
) -> Result<Option<(PathBuf, InstallBaseOrigin)>> {
    let applying = ConfigLayers {
        own: UserConfig::load_own().unwrap_or_default(),
        own_path: UserConfig::default_path(),
        site: Some(InstalledSiteProfile {
            path: Files::resolve()?.profile,
            profile: prepared.profile.clone(),
        }),
    };
    Ok(applying.install_base(env.install_root_env.as_deref(), env.home.as_deref()))
}

/// Record the PATH step's outcome in the receipt (for `--show`).
fn record_path(prepared: &PreparedProfile, path: Option<&PathReport>, changed: bool) -> Result<()> {
    // Only a performed step is recorded: a dry run reaches no persister and
    // carries `planned` instead of `outcome`.
    let Some((observation, outcome)) = path.and_then(|path| {
        let environment = path.durable_environment.as_ref()?;
        let outcome = report::outcome_label(path.outcome.as_ref()?);
        Some(((path, environment), outcome))
    }) else {
        return Ok(());
    };
    let (path, environment) = observation;
    let files = Files::resolve()?;
    let Some(mut receipt) = SiteReceipt::load_from(&files.receipt)? else {
        return Ok(());
    };
    let entry = ReceiptPath {
        shim_dir: path.shim_dir.clone(),
        outcome: outcome.to_owned(),
        target: environment.target.clone(),
    };
    if receipt.path.as_ref() == Some(&entry) && !changed {
        return Ok(());
    }
    receipt.path = Some(entry);
    debug_assert_eq!(receipt.sha256, prepared.sha256);
    let existing = read_optional(&files.receipt)?;
    let text = receipt.render(existing.as_deref(), &files.receipt)?;
    if existing.as_deref() != Some(text.as_str()) {
        write_atomic(&files.receipt, text.as_bytes())?;
    }
    Ok(())
}

/// `vibe self bootstrap --site`: store the profile validated before the
/// install. PATH was handled by the bootstrap's own activation.
pub(in crate::commands::vvm) fn apply_after_bootstrap(
    ctx: &output::Context,
    env: &VvmEnv,
    prepared: &PreparedProfile,
) -> Result<()> {
    // The bootstrap installed and activated the instance itself; the report
    // still names the install root every later command will resolve.
    let install_root = applying_install_base(prepared, env)?;
    report::emit(&install_profile(prepared, false, false, install_root)?, ctx)
}

fn change(path: &Path, old: Option<&str>, new: Option<&str>, with_diff: bool) -> FileChange {
    let kind = match (old, new) {
        (None, Some(_)) => FileChangeKind::Create,
        (Some(old), Some(new)) if old == new => FileChangeKind::Unchanged,
        (Some(_), Some(_)) => FileChangeKind::Update,
        (Some(_), None) => FileChangeKind::Delete,
        (None, None) => FileChangeKind::Unchanged,
    };
    FileChange {
        path: path.display().to_string(),
        diff: (with_diff && kind != FileChangeKind::Unchanged)
            .then(|| line_diff(old.unwrap_or(""), new.unwrap_or(""))),
        change: kind,
    }
}

pub(super) fn read_optional(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading `{}`", path.display())),
    }
}

/// Write through a sibling temporary file and a rename, so a reader never
/// sees half a profile.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("`{}` has no parent directory", path.display()))?;
    std::fs::create_dir_all(parent).with_context(|| format!("creating `{}`", parent.display()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("creating a temporary file in `{}`", parent.display()))?;
    std::io::Write::write_all(&mut temporary, bytes)
        .with_context(|| format!("writing `{}`", path.display()))?;
    temporary
        .persist(path)
        .map_err(|error| anyhow!("writing `{}`: {}", path.display(), error.error))?;
    Ok(())
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
