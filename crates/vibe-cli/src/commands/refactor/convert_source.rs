//! Shared discovery, honesty, prompting, and write pipeline for source conversion.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-051#convert-source");

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_specdoc::{Conversion, convert, is_spec_source};

use crate::output;

use super::{ResolvedConversion, SourceFormat};

#[derive(Debug)]
pub(super) enum ConversionTarget {
    Path(PathBuf),
    Refused { path: PathBuf, reason: String },
}

#[derive(Debug)]
enum Discovered {
    Candidate,
    Already,
    SkippedGenerated,
    SkippedForeign,
    SkippedHarness,
    Refused(String),
}

#[derive(Debug, Default)]
struct Counts {
    converted: usize,
    already: usize,
    lossy_confirmed: usize,
    refused: usize,
    skipped_generated: usize,
    skipped_foreign: usize,
    skipped_harness: usize,
    dry_run: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LossDisposition {
    Convert,
    Prompt,
    Refuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptAnswer {
    Yes,
    No,
    All,
}

pub(super) fn run_conversion(
    ctx: &output::Context,
    targets: Vec<ConversionTarget>,
    conversion: ResolvedConversion,
    command_name: &str,
) -> Result<()> {
    let mut discovered = BTreeMap::new();
    for target in targets {
        match target {
            ConversionTarget::Path(path) => {
                if let Err(error) = discover_argument(&path, &conversion, &mut discovered) {
                    discovered.insert(path, Discovered::Refused(format!("{error:#}")));
                }
            }
            ConversionTarget::Refused { path, reason } => {
                discovered.insert(path, Discovered::Refused(reason));
            }
        }
    }

    let attended = console::user_attended() && !ctx.is_json() && !ctx.is_unattended();
    let mut accept_all = false;
    let mut counts = Counts::default();
    for (path, item) in discovered {
        match item {
            Discovered::Already => {
                println!("already {}", path.display());
                counts.already += 1;
            }
            Discovered::SkippedGenerated => {
                println!("skipped-generated {}", path.display());
                counts.skipped_generated += 1;
            }
            Discovered::SkippedForeign => {
                println!("skipped-foreign {}", path.display());
                counts.skipped_foreign += 1;
            }
            Discovered::SkippedHarness => {
                println!("skipped-harness {}", path.display());
                counts.skipped_harness += 1;
            }
            Discovered::Refused(reason) => {
                report_refusal(&path, &reason, conversion.dry_run);
                counts.refused += 1;
            }
            Discovered::Candidate => {
                process_candidate(&path, &conversion, attended, &mut accept_all, &mut counts)
            }
        }
    }
    print_summary(&counts);

    if conversion.dry_run {
        Ok(())
    } else if counts.refused > 0 {
        bail!("{command_name} refused {} file(s)", counts.refused)
    } else {
        Ok(())
    }
}

fn discover_argument(
    path: &Path,
    conversion: &ResolvedConversion,
    discovered: &mut BTreeMap<PathBuf, Discovered>,
) -> Result<()> {
    if !path.exists() {
        discovered.insert(
            path.to_path_buf(),
            Discovered::Refused("path does not exist".to_string()),
        );
        return Ok(());
    }
    if path_has_vibedeps(path) {
        discovered.insert(
            path.to_path_buf(),
            Discovered::Refused("vibedeps is never converted".to_string()),
        );
        return Ok(());
    }
    if path.is_file() && path_has_node_modules(path) {
        discovered.insert(
            path.to_path_buf(),
            Discovered::Refused("node_modules is foreign and never converted".to_string()),
        );
        return Ok(());
    }
    if path.is_file() && is_harness_contract(path) {
        discovered.insert(
            path.to_path_buf(),
            Discovered::Refused("harness contract is never converted".to_string()),
        );
        return Ok(());
    }
    if path.is_file() {
        let extension = path.extension().and_then(|value| value.to_str());
        if extension == Some(conversion.to.extension()) {
            discovered.insert(path.to_path_buf(), Discovered::Already);
        } else if extension == Some(conversion.from.extension()) {
            // An explicit file overrides only the generated marker skip.
            discovered.insert(path.to_path_buf(), Discovered::Candidate);
        } else {
            discovered.insert(
                path.to_path_buf(),
                Discovered::Refused("file is not a selected md/xml spec source".to_string()),
            );
        }
        return Ok(());
    }
    if path.is_dir() {
        if path_has_node_modules(path) {
            discovered.insert(path.to_path_buf(), Discovered::SkippedForeign);
            return Ok(());
        }
        return walk_directory(path, conversion, discovered);
    }
    discovered.insert(
        path.to_path_buf(),
        Discovered::Refused("path is neither a regular file nor a directory".to_string()),
    );
    Ok(())
}

fn walk_directory(
    directory: &Path,
    conversion: &ResolvedConversion,
    discovered: &mut BTreeMap<PathBuf, Discovered>,
) -> Result<()> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("reading directory `{}`", directory.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("enumerating directory `{}`", directory.display()))?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("reading file type for `{}`", path.display()))?;
        if file_type.is_dir() {
            if is_foreign_directory(&path) {
                discovered.entry(path).or_insert(Discovered::SkippedForeign);
                continue;
            }
            if should_skip_directory(&path) {
                continue;
            }
            walk_directory(&path, conversion, discovered)?;
        } else if file_type.is_file() && is_spec_source(&path) {
            if is_harness_contract(&path) {
                discovered.entry(path).or_insert(Discovered::SkippedHarness);
                continue;
            }
            let extension = path.extension().and_then(|value| value.to_str());
            if extension == Some(conversion.from.extension()) {
                let source = fs::read_to_string(&path)
                    .with_context(|| format!("reading `{}`", path.display()))?;
                let item = if has_generated_marker(&source) {
                    Discovered::SkippedGenerated
                } else {
                    Discovered::Candidate
                };
                discovered.entry(path).or_insert(item);
            } else if extension == Some(conversion.to.extension()) {
                discovered.entry(path).or_insert(Discovered::Already);
            }
        }
    }
    Ok(())
}

fn should_skip_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    name == "target" || name == "vibedeps" || name.starts_with('.')
}

fn is_foreign_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name == "node_modules")
}

fn path_has_vibedeps(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "vibedeps")
}

fn path_has_node_modules(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "node_modules")
}

fn is_harness_contract(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                "CLAUDE.md" | "AGENTS.md" | "GEMINI.md" | "MEMORY.md" | "SKILL.md"
            )
        })
}

fn has_generated_marker(source: &str) -> bool {
    source
        .lines()
        .take(3)
        .any(|line| line.to_ascii_lowercase().contains("generated by vibe"))
}

fn process_candidate(
    path: &Path,
    conversion: &ResolvedConversion,
    attended: bool,
    accept_all: &mut bool,
    counts: &mut Counts,
) {
    let sibling = path.with_extension(conversion.to.extension());
    if sibling.exists() {
        report_refusal(
            path,
            &format!(
                "target `{}` already exists; refusing a paired document",
                sibling.display()
            ),
            conversion.dry_run,
        );
        counts.refused += 1;
        return;
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            report_refusal(path, &format!("read failed: {error}"), conversion.dry_run);
            counts.refused += 1;
            return;
        }
    };
    let converted = match convert(&source, conversion.to.direction()) {
        Ok(converted) => converted,
        Err(error) => {
            report_refusal(
                path,
                &format!("source parse failed: {error}"),
                conversion.dry_run,
            );
            counts.refused += 1;
            return;
        }
    };

    match converted {
        Conversion::ByteStable { output } => {
            if conversion.dry_run {
                println!("dry-run byte-stable {}", path.display());
                counts.dry_run += 1;
            } else {
                write_or_refuse(path, &output, conversion.to, "converted", counts, false);
            }
        }
        Conversion::IrStableLoss { output, loss } => process_lossy(
            path, &output, &loss, conversion, attended, accept_all, counts,
        ),
        Conversion::IrDivergent { detail } => {
            if conversion.dry_run {
                println!("dry-run ir-divergent {}: {detail}", path.display());
            } else {
                report_refusal(
                    path,
                    &format!("IR-divergent: {detail}; --force does not apply"),
                    false,
                );
            }
            counts.refused += 1;
        }
    }
}

fn process_lossy(
    path: &Path,
    output: &str,
    loss: &str,
    conversion: &ResolvedConversion,
    attended: bool,
    accept_all: &mut bool,
    counts: &mut Counts,
) {
    if conversion.dry_run {
        println!("dry-run ir-stable-loss {}\n{loss}", path.display());
        counts.dry_run += 1;
        return;
    }

    let disposition = loss_disposition(conversion.force, attended, *accept_all);
    let approved = match disposition {
        LossDisposition::Convert => {
            println!("loss {}\n{loss}", path.display());
            true
        }
        LossDisposition::Refuse => false,
        LossDisposition::Prompt => {
            eprintln!("lossy {}\n{loss}", path.display());
            match prompt_for_loss(path) {
                Ok(PromptAnswer::Yes) => true,
                Ok(PromptAnswer::All) => {
                    *accept_all = true;
                    true
                }
                Ok(PromptAnswer::No) => false,
                Err(error) => {
                    eprintln!("refused {}: {error:#}", path.display());
                    false
                }
            }
        }
    };
    if approved {
        write_or_refuse(path, output, conversion.to, "lossy-confirmed", counts, true);
    } else {
        println!("refused {}", path.display());
        if disposition == LossDisposition::Refuse {
            eprintln!("lossy {}\n{loss}", path.display());
        }
        counts.refused += 1;
    }
}

fn loss_disposition(force: bool, attended: bool, accept_all: bool) -> LossDisposition {
    if force || accept_all {
        LossDisposition::Convert
    } else if attended {
        LossDisposition::Prompt
    } else {
        LossDisposition::Refuse
    }
}

fn prompt_for_loss(path: &Path) -> Result<PromptAnswer> {
    loop {
        print!("Convert lossy file {}? [yes/no/all] ", path.display());
        io::stdout().flush().context("flushing conversion prompt")?;
        let mut answer = String::new();
        let read = io::stdin()
            .read_line(&mut answer)
            .context("reading conversion confirmation")?;
        if read == 0 {
            return Ok(PromptAnswer::No);
        }
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(PromptAnswer::Yes),
            "n" | "no" => return Ok(PromptAnswer::No),
            "a" | "all" => return Ok(PromptAnswer::All),
            _ => eprintln!("answer yes, no, or all"),
        }
    }
}

fn write_or_refuse(
    path: &Path,
    output: &str,
    target: SourceFormat,
    status: &str,
    counts: &mut Counts,
    lossy: bool,
) {
    match write_conversion(path, output, target) {
        Ok(()) => {
            println!("{status} {}", path.display());
            if lossy {
                counts.lossy_confirmed += 1;
            } else {
                counts.converted += 1;
            }
        }
        Err(error) => {
            println!("refused {}", path.display());
            eprintln!("refused {}: {error:#}", path.display());
            counts.refused += 1;
        }
    }
}

fn write_conversion(path: &Path, output: &str, target: SourceFormat) -> Result<()> {
    let sibling = path.with_extension(target.extension());
    if sibling.exists() {
        bail!(
            "target `{}` already exists; refusing to overwrite a paired document",
            sibling.display()
        );
    }
    fs::write(&sibling, output)
        .with_context(|| format!("writing target `{}`", sibling.display()))?;
    if let Err(remove_error) = fs::remove_file(path) {
        return match fs::remove_file(&sibling) {
            Ok(()) => Err(remove_error).with_context(|| {
                format!("removing source `{}`; target rolled back", path.display())
            }),
            Err(rollback_error) => bail!(
                "removing source `{}` failed: {remove_error}; rolling back `{}` also failed: {rollback_error}",
                path.display(),
                sibling.display()
            ),
        };
    }
    Ok(())
}

fn report_refusal(path: &Path, reason: &str, dry_run: bool) {
    if dry_run {
        println!("dry-run refused {}: {reason}", path.display());
    } else {
        println!("refused {}", path.display());
        eprintln!("refused {}: {reason}", path.display());
    }
}

fn print_summary(counts: &Counts) {
    println!(
        "summary converted={} already={} lossy-confirmed={} refused={} skipped-generated={} skipped-foreign={} skipped-harness={} dry-run={}",
        counts.converted,
        counts.already,
        counts.lossy_confirmed,
        counts.refused,
        counts.skipped_generated,
        counts.skipped_foreign,
        counts.skipped_harness,
        counts.dry_run
    );
}

#[cfg(test)]
mod tests {
    use super::{LossDisposition, loss_disposition};

    #[test]
    fn loss_policy_uses_injected_attendance() {
        assert_eq!(
            loss_disposition(false, false, false),
            LossDisposition::Refuse
        );
        assert_eq!(
            loss_disposition(false, true, false),
            LossDisposition::Prompt
        );
        assert_eq!(
            loss_disposition(true, false, false),
            LossDisposition::Convert
        );
        assert_eq!(
            loss_disposition(false, false, true),
            LossDisposition::Convert
        );
    }
}
