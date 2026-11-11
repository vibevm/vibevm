//! `cargo xtask opencode-bases` — the prompt-base sync tool of PROP-076
//! `#SYS-SUPPLIED-BASE`.
//!
//! **What it is for.** A harness whose only subagent-safe instruction slot
//! *replaces* its base prompt can only be given a boot lane if vibe supplies
//! that base, and the base is therefore versioned data with provenance in
//! that harness's own vibe package. This command is what puts a new version's
//! data there, and — far more importantly — it is the **only** thing that may
//! write `verified = true`.
//!
//! **Two outcomes, and they mean different things.**
//!
//! - *Fingerprints unchanged* — every assembly file upstream still has the
//!   digest the previous set recorded. The new version's base files are
//!   written, the provenance is carried forward with the new tag and digests,
//!   the whole-request proof is run offline against the installed binary, and
//!   `verified` is set from its verdict.
//! - *Fingerprints changed* — it **refuses** and prints a diff of the changed
//!   assembly files. A human then answers one question: does the change
//!   affect the composition this provenance describes? A change the recorded
//!   data can express (another selection row, another bullet) is an edit to
//!   the provenance and a re-run; a change it cannot express is a deliberate
//!   extension of vibe's interpreter with its own spec amendment. Neither is
//!   a job for a tool, which is exactly why this one stops.
//!
//! **Never a hand-edit.** `verified = true` written by a person is a guessed
//! base wearing a measurement's clothes, and a guessed base is the silent
//! failure the whole mechanism exists to prevent. The flag is written here,
//! from a verdict, or it is not written.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_workspace::system_prompt::{Launch, PROVENANCE_FILE, Provenance};

/// The model the offline proof runs under, which is the one its loopback
/// provider declares. It must match `opencode-prove.sh`'s own configuration:
/// a model id matching no selection row, so the proof measures the default
/// base — the one every other row is an exception to.
const PROOF_MODEL: &str = "fake-1";

/// The display name that provider gives it, for a substitution that asks.
const PROOF_MODEL_NAME: &str = "Fake 1";

/// Where the upstream files are fetched from, as a format over the tag.
const RAW_BASE: &str = "https://raw.githubusercontent.com";

/// The arguments of the command.
#[derive(Debug, clap::Args)]
pub struct OpencodeBasesArgs {
    /// The harness version to produce a set for — `2.2.8` for OpenCode
    /// 2.2.8. The upstream tag is `v<version>` unless `--tag` says
    /// otherwise.
    #[arg(long)]
    pub version: String,

    /// The package checkout to write into. Its `prompt-bases/` directory is
    /// where the sets live.
    #[arg(long)]
    pub package: PathBuf,

    /// The upstream tag, when it is not `v<version>`.
    #[arg(long)]
    pub tag: Option<String>,

    /// The set to carry the description forward from. Defaults to the
    /// newest one already in the package.
    #[arg(long)]
    pub from: Option<String>,

    /// The sample project the proof runs in, made by
    /// `campaigns/sys-prompt-2026-11/live/make-sample.sh`.
    #[arg(long)]
    pub sample: PathBuf,

    /// Where the proof's captures are written. Must not exist.
    #[arg(long)]
    pub out: PathBuf,

    /// Prove the set that is already in the package rather than fetching a
    /// new one. The fetch half needs the network and the upstream tag; the
    /// proof half needs only the installed binary, which is why re-proving
    /// an existing set is a separate run.
    #[arg(long)]
    pub prove_only: bool,

    /// The OpenCode binary the proof runs. Its `--version` must be the
    /// version under test, because a proof against another build proves
    /// nothing about this one.
    #[arg(long, default_value = "opencode")]
    pub opencode: String,
}

/// Run the sync tool.
pub fn run(args: OpencodeBasesArgs) -> Result<()> {
    let set = args.package.join("prompt-bases").join(&args.version);
    if !args.prove_only {
        fetch_and_compare(&args, &set)?;
    }
    if !set.join(PROVENANCE_FILE).is_file() {
        bail!(
            "`{}` holds no `{PROVENANCE_FILE}`, so there is nothing to prove. Run without \
             `--prove-only` to fetch the set first",
            set.display()
        );
    }
    // **The installed binary must be the version under test.** A proof is a
    // measurement of one program; running it against another build would
    // record a verdict about something nobody asked about.
    let reported = probe_version(&args.opencode)?;
    if !reported.contains(&args.version) {
        bail!(
            "`{} --version` says `{reported}` and the set under test is {}. The proof is a \
             measurement of ONE program: install that version, or pass `--opencode <path>`",
            args.opencode,
            args.version
        );
    }
    let provenance = Provenance::read(&set)
        .map_err(|error| anyhow::anyhow!("reading `{}`: {error}", set.display()))?;
    // Every digest, including the files this model does not select: a set is
    // data this package redistributes whole.
    let sizes = provenance
        .verify_all(&set)
        .map_err(|error| anyhow::anyhow!("the set's own integrity: {error}"))?;
    println!("set            : {}", set.display());
    println!("files verified : {} ({sizes:?})", sizes.len());

    let lane = args.sample.join("vibevm/vibespecs/boot/AGENT-LANE.md");
    if !lane.is_file() {
        bail!(
            "`{}` is not there — build the sample with \
             `campaigns/sys-prompt-2026-11/live/make-sample.sh <dir> 202349` first",
            lane.display()
        );
    }
    // The prover's own door, and the only caller of it: a set that has not
    // been proved yet cannot be composed through the launch entry, because
    // that entry's whole job is to refuse an unproved set.
    let tools = provenance.gate_tools();
    let base = provenance
        .compose_unverified(
            &set,
            &Launch {
                model_id: PROOF_MODEL,
                model_name: PROOF_MODEL_NAME,
                tools: &tools,
            },
        )
        .map_err(|error| anyhow::anyhow!("composing the base for the proof: {error}"))?;
    let document = provenance
        .carrier_document(&base, &lane)
        .map_err(|error| anyhow::anyhow!("composing the carrier document: {error}"))?;
    println!(
        "base composed  : {} bytes, sha256 {}",
        base.len(),
        hex(base.as_bytes())
    );

    std::fs::create_dir_all(&args.out)
        .with_context(|| format!("creating `{}`", args.out.display()))?;
    let carrier = args.out.join("carrier.json");
    std::fs::write(
        &carrier,
        serde_json::to_vec_pretty(&serde_json::json!({
            "env": document.env,
            "value": document.value,
            "flags": document.flags,
        }))?,
    )
    .with_context(|| format!("writing `{}`", carrier.display()))?;

    let captures = args.out.join("captures");
    let verdict = prove(&args, &captures, &carrier)?;
    if !verdict {
        bail!(
            "the whole-request proof FAILED — `verified` is not written. The report is \
             `{}/report.txt`, and every condition it failed is named in it",
            captures.display()
        );
    }
    let proof_id = proof_id(&captures)?;
    write_verified(&set, &proof_id, &today()?)?;
    println!(
        "\nverified       : true, proof `{proof_id}` — written into `{}`",
        set.join(PROVENANCE_FILE).display()
    );
    Ok(())
}

/// Fetch the upstream files for a new version and refuse when any assembly
/// file changed.
///
/// The fingerprints are **drift tripwires**, not proof: they catch an
/// upstream edit to anything that could change the composition the previous
/// provenance describes, and they cannot catch a published bundle that
/// disagrees with its own tag. That is what the proof is for.
fn fetch_and_compare(args: &OpencodeBasesArgs, set: &Path) -> Result<()> {
    let previous = match &args.from {
        Some(named) => args.package.join("prompt-bases").join(named),
        None => newest_set(&args.package.join("prompt-bases"), &args.version)?,
    };
    let reference = Provenance::read(&previous)
        .map_err(|error| anyhow::anyhow!("reading `{}`: {error}", previous.display()))?;
    let tag = args
        .tag
        .clone()
        .unwrap_or_else(|| format!("v{}", args.version));
    let repository = reference
        .upstream
        .as_ref()
        .map(|upstream| upstream.repository.clone())
        .unwrap_or_default();
    let slug = repository
        .trim_end_matches('/')
        .rsplit("github.com/")
        .next()
        .unwrap_or_default()
        .to_owned();
    if slug.is_empty() {
        bail!(
            "`{}` records no `[upstream] repository` on github.com, so there is nowhere to \
             fetch from",
            previous.join(PROVENANCE_FILE).display()
        );
    }
    println!("fetching       : {slug} at {tag}");

    // The tripwires first, and a refusal before a single base file is
    // written: a set half-produced from a changed upstream is worse than no
    // set, because it looks finished.
    let mut changed = Vec::new();
    for fingerprint in &reference.fingerprint {
        let Some(recorded) = fingerprint.sha256.as_deref() else {
            continue;
        };
        let bytes = fetch(&slug, &tag, &fingerprint.path)?;
        let found = hex(&bytes);
        if !found.eq_ignore_ascii_case(recorded) {
            changed.push((fingerprint.path.clone(), fingerprint.decides.clone(), bytes));
        }
    }
    if !changed.is_empty() {
        println!(
            "\n{} assembly file(s) changed between {} and {tag}:",
            changed.len(),
            reference.harness_version
        );
        for (path, decides, bytes) in &changed {
            println!("\n--- {path}");
            println!("    decides: {decides}");
            let old =
                fetch(&slug, &format!("v{}", reference.harness_version), path).unwrap_or_default();
            print_diff(
                &String::from_utf8_lossy(&old),
                &String::from_utf8_lossy(bytes),
            );
        }
        bail!(
            "REFUSING to verify {}: the files that decide the composition changed upstream. \
             Read the diff above and answer one question — can the recorded data express the \
             change? If yes (another `[[select]]` row, another `[[render.bullet]]`), extend \
             `PROVENANCE.toml` and re-run. If no, extend vibe's interpreter first, with its \
             own spec amendment. Never hand-edit `verified`",
            args.version
        );
    }
    println!(
        "fingerprints   : all {} unchanged",
        reference.fingerprint.len()
    );

    std::fs::create_dir_all(set).with_context(|| format!("creating `{}`", set.display()))?;
    let mut rows = Vec::new();
    for base in &reference.base {
        let bytes = fetch(&slug, &tag, &base.upstream_path)?;
        std::fs::write(set.join(&base.file), &bytes)
            .with_context(|| format!("writing `{}`", set.join(&base.file).display()))?;
        rows.push((base.file.clone(), bytes.len(), hex(&bytes)));
    }
    if let Some(license) = reference.upstream.as_ref().and_then(|u| u.license.as_ref()) {
        let bytes = fetch(&slug, &tag, &license.upstream_path)?;
        std::fs::write(set.join(&license.file), &bytes)
            .with_context(|| format!("writing `{}`", set.join(&license.file).display()))?;
        rows.push((license.file.clone(), bytes.len(), hex(&bytes)));
    }
    let carried = carry_forward(
        &std::fs::read_to_string(previous.join(PROVENANCE_FILE))?,
        &args.version,
        &tag,
        &rows,
    );
    std::fs::write(set.join(PROVENANCE_FILE), carried)?;
    println!(
        "written        : {} file(s) into `{}`, with `verified = false` until the proof runs",
        rows.len(),
        set.display()
    );
    Ok(())
}

/// One upstream file at one tag.
fn fetch(slug: &str, tag: &str, path: &str) -> Result<Vec<u8>> {
    let url = format!("{RAW_BASE}/{slug}/{tag}/{path}");
    let answer = reqwest::blocking::Client::builder()
        .build()?
        .get(&url)
        .send()
        .with_context(|| format!("fetching `{url}`"))?;
    if !answer.status().is_success() {
        bail!("`{url}` answered {}", answer.status());
    }
    Ok(answer.bytes()?.to_vec())
}

/// Carry the previous description forward: the version, the tag and every
/// recorded digest, with `verified` reset.
///
/// A textual rewrite rather than a serialisation, deliberately: the document
/// is written for a human and most of it is prose — the caveats, the
/// `decides` lines, the reason each selection row is where it is. Round-
/// tripping it through a serialiser would throw all of that away and leave a
/// reader with a table nobody can audit.
fn carry_forward(
    previous: &str,
    version: &str,
    tag: &str,
    rows: &[(String, usize, String)],
) -> String {
    let mut out = String::with_capacity(previous.len());
    let mut file = None;
    for line in previous.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("harness_version = ") {
            let _ = rest;
            out.push_str(&format!("harness_version = \"{version}\"\n"));
            continue;
        }
        if trimmed.starts_with("tag = ") {
            out.push_str(&format!("tag = \"{tag}\"\n"));
            continue;
        }
        if trimmed.starts_with("commit = ") {
            out.push_str("commit = \"TODO: the commit this tag points at\"\n");
            continue;
        }
        if trimmed.starts_with("read_on = ") {
            out.push_str(&format!(
                "read_on = \"{}\"\n",
                today().unwrap_or_else(|_| "TODO".to_owned())
            ));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("file = \"") {
            file = rest.split('"').next().map(str::to_owned);
        }
        if let Some(name) = &file
            && let Some(row) = rows.iter().find(|(candidate, _, _)| candidate == name)
        {
            if trimmed.starts_with("bytes = ") {
                out.push_str(&format!("bytes = {}\n", row.1));
                continue;
            }
            if trimmed.starts_with("sha256 = ") {
                out.push_str(&format!("sha256 = \"{}\"\n", row.2));
                continue;
            }
        }
        if trimmed.starts_with("verified = ") {
            out.push_str("verified = false\n");
            continue;
        }
        if trimmed.starts_with("proof_id = ") {
            out.push_str("proof_id = \"\"\n");
            continue;
        }
        if trimmed.starts_with("proof_date = ") {
            out.push_str("proof_date = \"\"\n");
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The newest set already in the package, excluding the one being produced.
fn newest_set(dir: &Path, excluding: &str) -> Result<PathBuf> {
    let mut found: Vec<(semver::Version, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("reading `{}`", dir.display()))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == excluding {
            continue;
        }
        if let Ok(version) = semver::Version::parse(&name) {
            found.push((version, entry.path()));
        }
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    found.pop().map(|(_, path)| path).with_context(|| {
        format!(
            "`{}` holds no set to carry a description forward from; the \
                                  first set for a harness is written by hand, from the runbook \
                                  in that package's README",
            dir.display()
        )
    })
}

/// Ask the installed binary what it is.
fn probe_version(program: &str) -> Result<String> {
    let output = Command::new(program)
        .arg("--version")
        .output()
        .with_context(|| format!("running `{program} --version`"))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Run the proof script and answer whether it passed.
fn prove(args: &OpencodeBasesArgs, captures: &Path, carrier: &Path) -> Result<bool> {
    let script = Path::new("campaigns/sys-prompt-2026-11/live/opencode-prove.sh");
    if !script.is_file() {
        bail!(
            "`{}` is not there — this command runs from the vibevm repository root",
            script.display()
        );
    }
    println!("\nproving        : {} …", script.display());
    let status = Command::new("bash")
        .arg(script)
        .arg(&args.sample)
        .arg(captures)
        .arg(carrier)
        .arg(&args.opencode)
        .status()
        .with_context(|| format!("running `{}`", script.display()))?;
    Ok(status.success())
}

/// The proof id the report states.
fn proof_id(captures: &Path) -> Result<String> {
    let report = std::fs::read_to_string(captures.join("report.txt"))
        .with_context(|| format!("reading `{}`", captures.join("report.txt").display()))?;
    report
        .lines()
        .find_map(|line| line.split_once("proof id"))
        .map(|(_, rest)| rest.trim_start_matches([' ', ':']).trim().to_owned())
        .context("the proof's report states no proof id")
}

/// Write the earned flag, and nothing else, into the set's provenance.
fn write_verified(set: &Path, proof: &str, date: &str) -> Result<()> {
    let path = set.join(PROVENANCE_FILE);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("verified = ") {
            out.push_str("verified = true\n");
        } else if trimmed.starts_with("proof_id = ") {
            out.push_str(&format!("proof_id = \"{proof}\"\n"));
        } else if trimmed.starts_with("proof_date = ") {
            out.push_str(&format!("proof_date = \"{date}\"\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    std::fs::write(&path, out).with_context(|| format!("writing `{}`", path.display()))
}

/// Today, as the provenance spells a date.
fn today() -> Result<String> {
    let output = Command::new("date").arg("+%Y-%m-%d").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// One lowercase hexadecimal sha256.
fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// A line-oriented difference, enough for a human to answer the one question
/// the refusal asks.
fn print_diff(old: &str, new: &str) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut printed = 0;
    for (index, line) in new_lines.iter().enumerate() {
        if old_lines.get(index) != Some(line) {
            if printed == 0 {
                println!("    first differing line {}:", index + 1);
            }
            println!("    - {}", old_lines.get(index).unwrap_or(&"<absent>"));
            println!("    + {line}");
            printed += 1;
            if printed >= 20 {
                println!("    … {} more line(s) differ", new_lines.len() - index - 1);
                break;
            }
        }
    }
    if printed == 0 && old_lines.len() != new_lines.len() {
        println!(
            "    the file grew or shrank: {} line(s) → {} line(s)",
            old_lines.len(),
            new_lines.len()
        );
    }
}
