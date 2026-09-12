//! The command surface — every command of the binary and every flag it
//! accepts, walked out of `--help` (PROP-057 `##OBS-SURFACE-SNAPSHOTS`).
//!
//! It is taken from the help and not from the parser's source, and that
//! is a decision rather than a convenience. The surface a version
//! promises is the surface a READER meets: what `vibe doc build --help`
//! prints is what a person can type, and a flag the parser accepts but
//! the help hides is not part of the contract the documentation
//! describes.
//!
//! ## What is thrown away, and why the snapshot is width-independent
//!
//! A help screen is wrapped to a terminal width, and the width belongs to
//! the machine that ran it. So a summary is put back together from its
//! wrapped lines and reduced to single spaces before it enters a
//! snapshot. Without that, moving to a wider terminal would report every
//! command in the product as changed — the class of false alarm that
//! teaches a team to stop reading a report.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS");

use std::collections::BTreeSet;
use std::process::{Command, Stdio};

use vibe_wire::generated::doc_surface::{SurfaceCommand, SurfaceFlag};

use super::{SurfaceEnv, collapse};
use crate::error::{DocError, Result};

/// How deep the walk follows subcommands.
///
/// A bound rather than a hope: the walk drives a program whose help is
/// the program's own output, and a binary whose `foo --help` listed `foo`
/// again would otherwise be walked forever. Five is two levels deeper
/// than the deepest command this product has.
const MAX_DEPTH: usize = 5;

/// clap's own subcommand, which is help about help. It carries no surface
/// of its own and appears under every command, so walking it would fill a
/// snapshot with one repeated entry per command in the product.
const HELP_SUBCOMMAND: &str = "help";

/// Walk the binary's command tree.
///
/// The root comes first and its children follow in the order the help
/// lists them, which is the order a reader meets them.
pub fn tree(env: &SurfaceEnv) -> Result<Vec<SurfaceCommand>> {
    let root = program_name(env);
    let mut out = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    walk(env, &root, &[], String::new(), &mut out, &mut seen)?;
    Ok(out)
}

/// The name a reader types. Taken from the binary's file stem with the
/// platform's executable suffix already gone, so a snapshot recorded on
/// Windows and one recorded on Linux say the same word.
fn program_name(env: &SurfaceEnv) -> String {
    env.binary
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("vibe")
        .to_owned()
}

fn walk(
    env: &SurfaceEnv,
    root: &str,
    path: &[String],
    summary: String,
    out: &mut Vec<SurfaceCommand>,
    seen: &mut BTreeSet<String>,
) -> Result<()> {
    let spelling = if path.is_empty() {
        root.to_owned()
    } else {
        format!("{root} {}", path.join(" "))
    };
    if !seen.insert(spelling.clone()) {
        return Ok(());
    }
    let help = help_of(env, path, &spelling)?;
    let parsed = parse(&help);
    out.push(SurfaceCommand {
        path: spelling,
        summary,
        flags: parsed.flags,
    });
    if path.len() >= MAX_DEPTH {
        return Ok(());
    }
    for (name, child_summary) in parsed.subcommands {
        if name == HELP_SUBCOMMAND {
            continue;
        }
        let mut child = path.to_vec();
        child.push(name);
        walk(env, root, &child, child_summary, out, seen)?;
    }
    Ok(())
}

/// Run one `--help` and hand back what it printed.
fn help_of(env: &SurfaceEnv, path: &[String], spelling: &str) -> Result<String> {
    let fail = |message: String| DocError::Surface {
        message: format!("`{spelling} --help`: {message}"),
    };
    let output = Command::new(&env.binary)
        .args(path)
        .arg("--help")
        .stdin(Stdio::null())
        // The same three the `cli-help` generator strips, for the same
        // reason: help must not vary with the operator's terminal, their
        // colour preference, or a harness that stamps its name on the
        // envelope.
        .env("NO_COLOR", "1")
        .env_remove("VIBE_INVOKED_BY")
        .env_remove("VIBETERM")
        .env_remove("VIBEFRAME")
        .output()
        .map_err(|e| fail(format!("cannot run `{}`: {e}", env.binary.display())))?;
    if !output.status.success() {
        return Err(fail(format!(
            "exited {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"))
}

/// One help screen, read.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Help {
    /// `(name, summary)` per subcommand, in the order the help lists
    /// them.
    pub subcommands: Vec<(String, String)>,
    pub flags: Vec<SurfaceFlag>,
}

/// Read a help screen.
///
/// The grammar is the one clap prints and nothing more: a section opens
/// with an unindented `Word:` line, an entry starts at two spaces of
/// indent, and anything indented further belongs to the entry above it —
/// which is how a summary that did not fit the terminal comes back
/// together.
///
/// ```
/// let help = vibe_doc::surface::commands::parse(concat!(
///     "Do a thing.\n\n",
///     "Usage: vibe doc [OPTIONS] <COMMAND>\n\n",
///     "Commands:\n",
///     "  build  Render a documentation\n",
///     "         package\n",
///     "  help   Print this message\n\n",
///     "Options:\n",
///     "      --format <FORMAT>  Which projection\n",
///     "  -h, --help             Print help\n",
/// ));
/// assert_eq!(help.subcommands[0].0, "build");
/// assert_eq!(help.subcommands[0].1, "Render a documentation package");
/// assert_eq!(help.flags[0].name, "--format");
/// assert_eq!(help.flags[0].value, "FORMAT");
/// assert_eq!(help.flags[1].name, "--help");
/// assert_eq!(help.flags[1].value, "");
/// ```
pub fn parse(help: &str) -> Help {
    let mut out = Help::default();
    let mut section = Section::None;
    for entry in entries(help, &mut section) {
        match entry.0 {
            Section::Commands => {
                if let Some(command) = subcommand(&entry.1) {
                    out.subcommands.push(command);
                }
            }
            Section::Options => {
                if let Some(flag) = option(&entry.1) {
                    out.flags.push(flag);
                }
            }
            Section::None => {}
        }
    }
    out
}

/// Which list an entry belongs to. Everything else a help screen carries
/// — the about, the usage line, the arguments a command takes — is not
/// part of this surface: an argument's NAME is already in the usage the
/// command's own page shows, and a snapshot that recorded it would report
/// a change every time a placeholder was reworded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Commands,
    Options,
}

/// The column a subcommand's name stands in.
const COMMAND_COLUMN: usize = 2;

/// The deepest column a flag's own spelling stands in. clap indents a
/// long-only option by the width of the short slot it does not use, so
/// `-h, --help` starts at two and `      --json` at six — both are
/// entries, and both are far shallower than the ten a summary printed on
/// its own line starts at. The bound is what tells the two apart: a
/// possible-values list under an option begins with `-` as well, and
/// without it every option after the first would be swallowed into the
/// first one's summary.
const FLAG_COLUMN: usize = 7;

/// Does this line open a new entry, or continue the one above it?
fn opens_entry(section: Section, indent: usize, trimmed: &str) -> bool {
    match section {
        Section::Commands => indent <= COMMAND_COLUMN,
        Section::Options => indent <= FLAG_COLUMN && trimmed.starts_with('-'),
        Section::None => false,
    }
}

/// Split a help screen into `(section, entry text)` pairs, each entry
/// already unwrapped into one line.
fn entries(help: &str, section: &mut Section) -> Vec<(Section, String)> {
    let mut out: Vec<(Section, String)> = Vec::new();
    for line in help.split('\n') {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent == 0 {
            *section = match trimmed {
                "Commands:" => Section::Commands,
                "Options:" => Section::Options,
                _ => Section::None,
            };
            continue;
        }
        if *section == Section::None {
            continue;
        }
        if opens_entry(*section, indent, trimmed) {
            out.push((*section, trimmed.to_owned()));
            continue;
        }
        // A line that opens nothing continues the entry above it, and
        // there is nothing above it until a section has opened one.
        let Some(last) = out.last_mut() else {
            continue;
        };
        if last.0 != *section {
            continue;
        }
        // Two spaces or one, and the difference is the whole point. An
        // entry that has no column break yet is one clap printed with the
        // help on the NEXT line, so what follows is its summary and the
        // break has to be put in; an entry that already has one is a
        // summary the terminal wrapped, and the pieces join with a single
        // space.
        last.1
            .push_str(if last.1.contains("  ") { " " } else { "  " });
        last.1.push_str(trimmed);
    }
    out
}

/// `build  Render a package` → `("build", "Render a package")`.
fn subcommand(entry: &str) -> Option<(String, String)> {
    let (head, rest) = split_columns(entry);
    // clap prints aliases as `name, alias`; the name is the first.
    let name = head.split(',').next().unwrap_or(head).trim();
    if name.is_empty() || name.starts_with('-') {
        return None;
    }
    Some((name.to_owned(), collapse(rest)))
}

/// `-f, --format <FORMAT>  Which projection` → the flag.
fn option(entry: &str) -> Option<SurfaceFlag> {
    let (head, rest) = split_columns(entry);
    if !head.starts_with('-') {
        return None;
    }
    let mut name = String::new();
    let mut value = String::new();
    for part in head.split(',') {
        let part = part.trim();
        // The placeholder travels with whichever spelling carries it, so
        // it is read here rather than from the tail of the entry.
        let (spelling, placeholder) = match part.split_once(char::is_whitespace) {
            Some((spelling, placeholder)) => (spelling, placeholder.trim()),
            None => (part, ""),
        };
        if !placeholder.is_empty() {
            value = placeholder
                .trim_matches(['<', '>', '[', ']'].as_slice())
                .to_owned();
        }
        // The long form is the one a reader writes down and the one a
        // page quotes; the short is recorded only when there is no long.
        if spelling.starts_with("--") || name.is_empty() {
            name = spelling.to_owned();
        }
    }
    if name.is_empty() {
        return None;
    }
    Some(SurfaceFlag {
        name,
        value,
        summary: collapse(rest),
    })
}

/// Split an entry into its first column and its summary. clap separates
/// the two by two or more spaces, which is what makes `-f, --format
/// <FORMAT>` one column rather than four.
fn split_columns(entry: &str) -> (&str, &str) {
    match entry.find("  ") {
        Some(at) => (entry[..at].trim_end(), entry[at..].trim_start()),
        None => (entry.trim_end(), ""),
    }
}

#[cfg(test)]
mod tests;
