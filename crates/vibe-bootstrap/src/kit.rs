//! Kit discovery and native-child orchestration. Never a second site-profile writer.
use crate::{file_verify, regular, verify_bundle};
use anyhow::{Context, Result, bail};
use clap::Parser;
use std::{
    fs,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::Instant,
};

/// Existing native-consumer filenames for one admitted distribution target.
/// This describes kit bytes, not the OS of the producer.
pub fn kit_layout(target: &str) -> Result<(String, &'static str)> {
    use vibe_wire::behaviour::release_manifest::SUPPORTED_DISTRIBUTION_TARGETS;
    anyhow::ensure!(
        SUPPORTED_DISTRIBUTION_TARGETS.contains(&target),
        "unsupported kit distribution target"
    );
    let (suffix, platform) = if target == "x86_64-pc-windows-msvc" {
        (".exe", "windows")
    } else if target.ends_with("-apple-darwin") {
        ("", "macos")
    } else {
        ("", "linux")
    };
    Ok((format!("vibe-installer-{target}{suffix}"), platform))
}

#[derive(Debug, Parser)]
#[command(
    name = "vibe-installer",
    version,
    about = "Install Vibe from this local corporate kit, without shell scripts"
)]
pub struct Args {
    #[arg(long)]
    pub kit: Option<PathBuf>,
    #[arg(long)]
    pub profile: Option<PathBuf>,
    #[arg(long,default_value="auto",value_parser=["auto","manual","pac"])]
    pub mode: String,
    #[arg(long)]
    pub bundle: Option<PathBuf>,
    #[arg(long)]
    pub destination: Option<PathBuf>,
    #[arg(long)]
    pub settings: Option<PathBuf>,
    #[arg(long, conflicts_with = "no_path")]
    pub fix_path: bool,
    /// Keep persistent user PATH unchanged (default: activate user PATH automatically).
    #[arg(long)]
    pub no_path: bool,
    /// Offer metadata-only configuration of existing AI connections after installation.
    #[arg(long)]
    pub configure_ai: bool,
    /// Explicitly upgrade an existing installation to this strictly newer verified kit.
    /// Requires --destination and --no-path; unattended alone never upgrades.
    #[arg(long, requires_all = ["destination", "no_path"], conflicts_with = "configure_ai")]
    pub upgrade: bool,
    #[arg(long)]
    pub unattended: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, short)]
    pub verbose: bool,
    #[arg(long)]
    pub debug: bool,
    #[arg(long)]
    pub log: Option<PathBuf>,
}

struct Reporter {
    verbose: bool,
    log: Option<fs::File>,
    start: Instant,
}
impl Reporter {
    fn stage(&mut self, name: &str) -> Result<()> {
        eprintln!("  · {name}");
        if let Some(log) = &mut self.log {
            writeln!(
                log,
                "stage={name};elapsed_ms={}",
                self.start.elapsed().as_millis()
            )?;
        }
        Ok(())
    }
    fn child(&mut self, command: &mut Command, phase: &str) -> Result<()> {
        self.stage(phase)?;
        let animation = io::stderr().is_terminal() && !self.verbose;
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let spinner = if animation {
            let done = done.clone();
            Some(std::thread::spawn(move || {
                let frames = ['·', '•', '●', '•'];
                let mut i = 0;
                while !done.load(std::sync::atomic::Ordering::Relaxed) {
                    eprint!("\r    {} working", frames[i % frames.len()]);
                    i += 1;
                    std::thread::sleep(std::time::Duration::from_millis(120));
                }
                eprint!("\r                    \r");
            }))
        } else {
            None
        };
        let outcome = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(spinner) = spinner {
            let _ = spinner.join();
        }
        let result = outcome.context("starting verified native vibe")?;
        if let Some(log) = &mut self.log {
            writeln!(
                log,
                "child_stage={phase};exit={:?};stdout_bytes={};stderr_bytes={};elapsed_ms={}",
                result.status.code(),
                result.stdout.len(),
                result.stderr.len(),
                self.start.elapsed().as_millis()
            )?;
        }
        // Only closed, value-free cause classifications leave child output.
        // Profile values, endpoint addresses and environment are never dumped.
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let reason = safe_cause(&stderr);
            if let Some(log) = &mut self.log {
                writeln!(
                    log,
                    "safe_error_identifiers={:?}",
                    safe_identifiers(&stderr)
                )?;
            }
            if let Some(log) = &mut self.log {
                writeln!(log, "failure_stage={phase};cause={reason}")?;
            }
            bail!(
                "native {phase} failed (exit {:?}): {reason}; earlier completed stages remain committed",
                result.status.code()
            );
        }
        if self.verbose {
            eprintln!("    native child finished successfully ({phase})");
        }
        Ok(())
    }
}

pub fn run(args: Args) -> Result<()> {
    let kit = args
        .kit
        .clone()
        .unwrap_or(
            std::env::current_exe()?
                .parent()
                .context("installer has no directory")?
                .to_path_buf(),
        )
        .canonicalize()
        .context("locating local kit")?;
    let mut report = Reporter {
        verbose: args.verbose || args.debug,
        log: None,
        start: Instant::now(),
    };
    if args.debug || args.log.is_some() {
        let path = args
            .log
            .clone()
            .unwrap_or(std::env::temp_dir().join(format!(
                "vibe-installer-{}-{}.log",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos()
            )));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        report.log = Some(
            options
                .open(&path)
                .context("creating private installer diagnostic log")?,
        );
        eprintln!(
            "Diagnostic log: {} (stage metadata only; no profile/environment dump)",
            path.display()
        );
    }
    eprintln!("Vibe · native local installer");
    report.stage("Validate local kit inputs")?;
    let (_, platform) = kit_layout(crate::target()?)?;
    let profile = args
        .profile
        .clone()
        .unwrap_or(kit.join(format!("profile-{platform}-{}.toml", args.mode)));
    let profile = if profile.is_absolute() {
        profile
    } else {
        kit.join(profile)
    };
    let text = file_verify::read_regular_bounded(&profile, 1024 * 1024)
        .context("reading chosen site profile")?;
    let table: toml::Value = toml::from_str(
        std::str::from_utf8(&text).context("site profile is not UTF-8")?,
    )
    .map_err(|_| anyhow::anyhow!("site profile is not valid TOML; contents omitted for privacy"))?;
    // Read only discovery fields. Full validation remains version-specific and
    // belongs to the verified native child before import/publication effects.
    let self_table = table.get("self");
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let expand = |value: &str| -> Result<PathBuf> {
        if value == "~" {
            return home.clone().context("profile needs a home directory");
        }
        if let Some(tail) = value
            .strip_prefix("~/")
            .or_else(|| value.strip_prefix("~\\"))
        {
            return Ok(home
                .clone()
                .context("profile needs a home directory")?
                .join(tail));
        }
        let p = PathBuf::from(value);
        if !p.is_absolute() {
            bail!("destination/settings path must be absolute or home-relative");
        }
        Ok(p)
    };
    for (explicit, key) in [
        (&args.destination, "install_root"),
        (&args.settings, "settings_dir"),
    ] {
        if let (Some(explicit), Some(declared)) = (
            explicit.as_ref(),
            self_table.and_then(|t| t.get(key)).and_then(|v| v.as_str()),
        ) && expand(&explicit.to_string_lossy())? != expand(declared)?
        {
            bail!(
                "explicit destination/settings conflicts with [self] {key}; choose a matching profile so current and persistent configuration agree"
            );
        }
    }
    let destination = args.destination.clone().or_else(|| {
        self_table
            .and_then(|t| t.get("install_root"))
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
    });
    let destination = destination
        .map(|p| expand(&p.to_string_lossy()))
        .transpose()?;
    let settings = args.settings.clone().or_else(|| {
        self_table
            .and_then(|t| t.get("settings_dir"))
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
    });
    let settings = settings.map(|p| expand(&p.to_string_lossy())).transpose()?;
    let checksum_bytes =
        file_verify::read_regular_bounded(&kit.join("SHA256SUMS.txt"), 4 * 1024 * 1024)
            .context("kit requires SHA256SUMS.txt")?;
    let sums = parse_checksums(std::str::from_utf8(&checksum_bytes)?)?;
    let target = crate::target()?;
    let candidates: Vec<_> = sums
        .iter()
        .filter(|(p, _)| {
            p.file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|v| v.starts_with("vibevm-") && v.ends_with(&format!("-{target}.zip")))
        })
        .collect();
    let selected = if let Some(path) = &args.bundle {
        let relative = path.strip_prefix(&kit).unwrap_or(path);
        sums.iter()
            .find(|(p, _)| p == relative)
            .context("chosen local bundle has no kit checksum")?
    } else {
        if candidates.len() != 1 {
            bail!("kit must name exactly one matching platform bundle, or pass --bundle");
        }
        candidates[0]
    };
    let bundle_path = kit.join(&selected.0);
    report.stage("Verify bundle, target, components and source")?;
    let verified = verify_bundle(&bundle_path, &selected.1, target)?;
    if let Some(log) = &mut report.log {
        writeln!(
            log,
            "verified_target={};release_version={};source_commit={};outer_sha256={};component_count={}",
            verified.manifest.target,
            verified.manifest.version,
            verified.manifest.source_commit,
            selected.1,
            verified.manifest.components.len()
        )?;
        for component in &verified.manifest.components {
            writeln!(
                log,
                "verified_component={};bytes={};digest={}",
                component.name.as_str(),
                component.size,
                component.digest
            )?;
        }
    }
    let executable = verified
        .directory
        .join(if cfg!(windows) { "vibe.exe" } else { "vibe" });
    // Reuse an already present, checksum-listed companion under the canonical
    // sibling name. No download or new companion resolver is introduced.
    let wants_px = table
        .get("proxy")
        .and_then(|v| v.get("mode"))
        .and_then(|v| v.as_str())
        == Some("px");
    if wants_px {
        let companion: Vec<_> = sums
            .iter()
            .filter(|(p, _)| {
                p.file_name().and_then(|v| v.to_str()).is_some_and(|n| {
                    n.starts_with("px-")
                        && (n.ends_with(&format!("-{target}.zip"))
                            || n.ends_with(&format!("-{target}.tar.gz")))
                })
            })
            .collect();
        if companion.len() != 1 {
            bail!(
                "profile requires a local px companion for this target; supply exactly one checksum-listed archive"
            );
        }
        let (relative, hash) = companion[0];
        let path = kit.join(relative);
        let (_, meta) = regular::open_regular_no_follow(&path)?;
        let got = file_verify::hash_regular_exact(&path, meta.len(), 256 * 1024 * 1024, false)?;
        if got != format!("sha256:{hash}") {
            bail!("local companion checksum mismatch");
        }
        let (mut input, measured) =
            file_verify::open_hashed_regular_exact(&path, meta.len(), 256 * 1024 * 1024, false)?;
        if measured != format!("sha256:{hash}") {
            bail!("local companion changed before staging");
        }
        let destination = verified
            .directory
            .join(relative.file_name().context("companion has no name")?);
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        file_verify::copy_bounded(&mut input, &mut output, meta.len())?;
    }
    let child = || {
        let mut c = Command::new(&executable);
        c.current_dir(&kit);
        if let Some(p) = &destination {
            c.env("VIBEVM_INSTALL_ROOT", p);
        }
        if let Some(p) = &settings {
            c.env("VIBE_SETTINGS", p);
        }
        c.arg("--unattended");
        c
    };
    // The corporate installer defaults to user-level PATH activation by the
    // owner's explicit policy. The general CLI consent rules are unchanged.
    let add_path = |c: &mut Command| {
        if args.no_path {
            c.arg("--no-fix-path");
        } else {
            c.arg("--fix-path");
        }
    };
    let mut preflight = child();
    preflight
        .args(["self", "apply"])
        .arg(&profile)
        .arg("--dry-run");
    add_path(&mut preflight);
    report.child(&mut preflight, "Preflight site profile (no publication)")?;
    if args.upgrade {
        crate::upgrade::run(
            &verified,
            &kit,
            destination
                .as_deref()
                .context("explicit upgrade destination absent")?,
            settings.as_deref(),
            args.dry_run,
        )?;
        report.stage(if args.dry_run {
            "Upgrade dry run complete; installation unchanged"
        } else {
            "Explicit newer kit upgrade complete; active payload verified"
        })?;
        return Ok(());
    }
    if args.dry_run {
        report.stage("Dry run complete; installation unchanged")?;
        return Ok(());
    }
    // self apply owns unmanaged import, activation, settings, companion and PATH
    // consent. Running it once avoids a second independently sequenced importer.
    let mut apply = child();
    apply.args(["self", "apply"]).arg(&profile);
    add_path(&mut apply);
    report.child(
        &mut apply,
        "Apply profile and activate verified distribution",
    )?;
    report.stage("Installation complete")?;
    if args.configure_ai
        && let Some(component) = verified.manifest.components.iter().find(|component| {
            component.name
                == vibe_wire::behaviour::release_manifest::DistributionComponentName::Vibe
        })
    {
        crate::optional_setup::after_install(
            &executable,
            component,
            &kit,
            destination.as_deref(),
            settings.as_deref(),
            args.unattended,
        );
    }
    Ok(())
}

pub fn parse_checksums(text: &str) -> Result<Vec<(PathBuf, String)>> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let (hash, name) = line
            .split_once(char::is_whitespace)
            .context("malformed kit checksum row")?;
        let name = name.trim_start().trim_start_matches('*');
        if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
            bail!("invalid kit SHA256");
        }
        let path = crate::source_extract::safe_source_path(name, false)
            .context("unsafe kit checksum path")?;
        if !seen.insert(crate::source_extract::collision_key(&path)) {
            bail!("duplicate kit checksum path");
        }
        out.push((path, hash.to_ascii_lowercase()));
    }
    Ok(out)
}

/// Closed diagnostic vocabulary; never relay arbitrary site-specific values.
fn safe_cause(stderr: &str) -> &'static str {
    let text = stderr.to_ascii_lowercase();
    for (needle, reason) in [
        (
            "permission denied",
            "permission denied; choose a writable per-user destination",
        ),
        (
            "access is denied",
            "access denied; choose a writable per-user destination",
        ),
        (
            "unknown field",
            "site profile contains an unsupported field; compare its schema with the bundled vibe version",
        ),
        (
            "checksum",
            "integrity validation failed; replace the damaged local artifact",
        ),
        (
            "digest",
            "integrity validation failed; replace the damaged local artifact",
        ),
        (
            "no space",
            "not enough disk space for the selected destination",
        ),
        (
            "not found",
            "a required local input or destination is unavailable",
        ),
        (
            "offline",
            "offline policy refused an unavailable local input",
        ),
        (
            "profile",
            "site profile validation or application failed; inspect the selected profile locally",
        ),
    ] {
        if text.contains(needle) {
            return reason;
        }
    }
    "native operation refused; inspect the selected destination and local artifact compatibility"
}

fn safe_identifiers(text: &str) -> Vec<String> {
    let allowed = [
        "registry_failed",
        "template_cache_missing",
        "permission_denied",
        "invalid_profile",
        "digest_mismatch",
        "artifact_missing",
        "source_unavailable",
        "need_unmet",
        "profile_invalid",
    ];
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|token| allowed.contains(token))
        .fold(Vec::new(), |mut values, token| {
            if !values.iter().any(|v| v == token) {
                values.push(token.to_owned());
            }
            values
        })
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn kit_layout_uses_distribution_targets_and_native_consumer_names() {
        for target in vibe_wire::behaviour::release_manifest::SUPPORTED_DISTRIBUTION_TARGETS {
            let (installer, platform) = kit_layout(target).unwrap();
            let suffix = if target == "x86_64-pc-windows-msvc" {
                ".exe"
            } else {
                ""
            };
            assert_eq!(installer, format!("vibe-installer-{target}{suffix}"));
            assert_eq!(
                platform,
                if target.contains("windows") {
                    "windows"
                } else if target.contains("apple") {
                    "macos"
                } else {
                    "linux"
                }
            );
        }
        assert_eq!(
            kit_layout("x86_64-unknown-linux-gnu").unwrap(),
            ("vibe-installer-x86_64-unknown-linux-gnu".into(), "linux")
        );
        assert!(kit_layout("riscv64-unknown-linux-gnu").is_err());
    }
    #[test]
    fn error_details_keep_codes_not_private_values() {
        let text =
            "registry_failed https://private.invalid/path?token=SECRET_SENTINEL permission denied";
        assert_eq!(safe_identifiers(text), vec!["registry_failed"]);
        assert!(!safe_cause(text).contains("private.invalid"));
        assert!(!format!("{:?}", safe_identifiers(text)).contains("SECRET_SENTINEL"));
    }
}
