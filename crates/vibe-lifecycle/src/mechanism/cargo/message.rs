//! The §5.0.3 FOREIGN-format readers — Cargo's `--message-format=json`
//! stream and `cargo metadata --format-version 1`.
//!
//! These are the one place in this crate where a handwritten
//! `Deserialize` is the right answer, and the reason is recorded in
//! §5.0.3: "The `--message-format=json-render-diagnostics` stream is
//! Cargo's wire, not ours: it is parsed with a minimal lenient serde shape
//! carrying exactly the members the laws read … No schema is authored for
//! another tool's format." Authoring a JTD schema here would freeze
//! another project's message format as ours, and a generated strict reader
//! would break the day Cargo adds a field — which it may, at any time,
//! without telling us.
//!
//! So the shapes below are deliberately **minimal and lenient**: exactly
//! `reason`, `package_id`, `target.{name,kind,crate_types}`, `executable`,
//! `filenames` and `fresh`,
//! with every other field ignored BY DESIGN. Leniency stops at the shape:
//! a line that is not a `reason`-tagged Cargo message refuses, because
//! that is the signal the format moved, and the alternative — falling back
//! to a guessed `target/<profile>/<name>` path — is exactly what §5's law
//! 3 forbids.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use serde::Deserialize;

use super::config::OutputSelect;
use crate::mechanism::MechanismError;
use crate::mechanism::error::preview;

/// The `reason` of the one message kind that names a produced artifact.
pub(crate) const COMPILER_ARTIFACT: &str = "compiler-artifact";

/// The Cargo target kind an executable artifact comes from.
const BIN_KIND: &str = "bin";

/// How many artifact names an ambiguity refusal spells.
const AMBIGUITY_PREVIEW: usize = 8;

/// One line of Cargo's JSON message stream, in the minimal lenient shape
/// the laws read. Unknown fields are ignored on purpose — this is another
/// tool's wire, and a strict reader would turn a Cargo release into a
/// vibe outage.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CargoMessage {
    /// The message tag. Required: an untagged line is not a Cargo message.
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) package_id: Option<String>,
    #[serde(default)]
    pub(crate) target: Option<CargoMessageTarget>,
    /// The produced executable, absolute. `null` for a library artifact —
    /// and the one honest source of an executable path there is.
    #[serde(default)]
    pub(crate) executable: Option<String>,
    /// Every artifact path Cargo associates with the target. Native cdylib
    /// selection uses this list and never derives a filename from a crate name.
    #[serde(default)]
    pub(crate) filenames: Vec<String>,
    /// Cargo's own freshness verdict for this artifact.
    #[serde(default)]
    pub(crate) fresh: Option<bool>,
}

/// The `target` member of a compiler-artifact message.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CargoMessageTarget {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) kind: Vec<String>,
    #[serde(default)]
    pub(crate) crate_types: Vec<String>,
}

/// `cargo metadata --format-version 1 --no-deps`, in the same posture.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CargoMetadata {
    #[serde(default)]
    pub(crate) packages: Vec<MetadataPackage>,
}

/// One workspace package as `cargo metadata` reports it.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MetadataPackage {
    #[serde(default)]
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) manifest_path: String,
    #[serde(default)]
    pub(crate) targets: Vec<MetadataTarget>,
}

/// One Cargo target of a workspace package.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MetadataTarget {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) kind: Vec<String>,
    #[serde(default)]
    pub(crate) crate_types: Vec<String>,
}

impl CargoMessage {
    /// Whether this message names a produced artifact of a `bin` target.
    fn is_executable_artifact(&self) -> bool {
        self.reason == COMPILER_ARTIFACT
            && self
                .target
                .as_ref()
                .is_some_and(|target| target.kind.iter().any(|kind| kind == BIN_KIND))
    }

    /// The artifact's `<package>#<target>` label, for a diagnostic.
    fn label(&self) -> String {
        let package = self.package_id.as_deref().unwrap_or("<no package_id>");
        let target = self.target.as_ref().map_or("<no target>", |t| &t.name);
        preview(&format!("{package}#{target}"))
    }
}

/// Read one complete `--message-format=json-render-diagnostics` stream.
///
/// Blank lines are skipped; every other line must decode. Rendered
/// diagnostics travel on stderr, so this stream is JSON end to end.
pub(crate) fn parse_stream(
    target: &str,
    stdout: &str,
) -> Result<Vec<CargoMessage>, MechanismError> {
    let mut messages = Vec::new();
    for (index, raw) in stdout.lines().enumerate() {
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<CargoMessage>(line) {
            Ok(message) => messages.push(message),
            Err(error) => {
                return Err(MechanismError::MessageDecode {
                    target: target.to_owned(),
                    line: index + 1,
                    reason: error.to_string(),
                    value: preview(line),
                });
            }
        }
    }
    Ok(messages)
}

/// Read one `cargo metadata --format-version 1` document.
pub(crate) fn parse_metadata(target: &str, stdout: &str) -> Result<CargoMetadata, MechanismError> {
    serde_json::from_str::<CargoMetadata>(stdout).map_err(|error| MechanismError::MetadataDecode {
        target: target.to_owned(),
        reason: error.to_string(),
    })
}

/// Whether a Cargo `package_id` names one package.
///
/// Three spellings are in the wild and all three are answered here rather
/// than by a substring test that would match `serde` inside `serde_json`:
/// the modern `PackageIdSpec` with a `name@version` fragment, the same
/// spec with a bare-version fragment (the name is then the URL's last
/// segment), and the legacy `name version (source)` triple.
pub(crate) fn package_id_names(package_id: &str, name: &str) -> bool {
    let Some((base, fragment)) = package_id.rsplit_once('#') else {
        return package_id.split_whitespace().next() == Some(name);
    };
    if let Some((spelled, _)) = fragment.rsplit_once('@') {
        return spelled == name;
    }
    if fragment.starts_with(|byte: char| byte.is_ascii_digit()) {
        // `…/vibe-helper#0.1.0` — the fragment is the version, so the
        // package name is the last path segment of the source URL.
        return base.rsplit(['/', '\\']).next().unwrap_or(base) == name;
    }
    fragment == name
}

/// Whether one artifact message answers an output's `select` predicate.
fn matches(message: &CargoMessage, select: &OutputSelect) -> bool {
    if let Some(package) = &select.package {
        let named = message
            .package_id
            .as_deref()
            .is_some_and(|id| package_id_names(id, package));
        if !named {
            return false;
        }
    }
    if let Some(bin) = &select.bin
        && message.target.as_ref().map(|target| target.name.as_str()) != Some(bin.as_str())
    {
        return false;
    }
    true
}

/// Select exactly one compiler-artifact message for one declared output.
///
/// The three refusals are the point of the function: zero matches, more
/// than one match, and a match whose `executable` is null are each a typed
/// refusal, and none of them falls back to a path derived from the target
/// directory. §5's law 3 says the artifact comes from the message stream
/// or it does not come at all.
pub(crate) fn select_message<'stream>(
    target: &str,
    output: &str,
    select: &OutputSelect,
    messages: &'stream [CargoMessage],
) -> Result<&'stream CargoMessage, MechanismError> {
    let considered: Vec<&CargoMessage> = messages
        .iter()
        .filter(|message| message.is_executable_artifact())
        .collect();
    let matched: Vec<&CargoMessage> = considered
        .iter()
        .copied()
        .filter(|message| matches(message, select))
        .collect();
    let predicate = select.describe();
    let [only] = matched[..] else {
        if matched.is_empty() {
            return Err(MechanismError::NoArtifact {
                target: target.to_owned(),
                output: output.to_owned(),
                predicate,
                considered: considered.len(),
            });
        }
        let kept = matched.len().min(AMBIGUITY_PREVIEW);
        let mut names: Vec<String> = matched[..kept]
            .iter()
            .map(|message| message.label())
            .collect();
        if kept < matched.len() {
            names.push(format!("and {} more", matched.len() - kept));
        }
        return Err(MechanismError::AmbiguousArtifact {
            target: target.to_owned(),
            output: output.to_owned(),
            predicate,
            matched: matched.len(),
            names: names.join(", "),
        });
    };
    if only.executable.is_none() {
        return Err(MechanismError::NoExecutable {
            target: target.to_owned(),
            output: output.to_owned(),
            bin: only
                .target
                .as_ref()
                .map_or_else(|| "<no target>".to_owned(), |t| preview(&t.name)),
        });
    }
    Ok(only)
}

/// Confirm an output's predicate against the resolved workspace before a
/// build runs — §5's law 1, so a typo refuses in a metadata call rather
/// than after a full compile.
pub(crate) fn confirm_against_metadata(
    target: &str,
    output: &str,
    select: &OutputSelect,
    metadata: &CargoMetadata,
) -> Result<(), MechanismError> {
    let packages: Vec<&MetadataPackage> = match &select.package {
        Some(name) => metadata
            .packages
            .iter()
            .filter(|package| &package.name == name)
            .collect(),
        None => metadata.packages.iter().collect(),
    };
    if let Some(name) = &select.package
        && packages.is_empty()
    {
        return Err(MechanismError::UnknownPackage {
            target: target.to_owned(),
            output: output.to_owned(),
            package: preview(name),
            candidates: names(metadata.packages.iter().map(|package| package.name.clone())),
        });
    }
    let Some(bin) = &select.bin else {
        return Ok(());
    };
    let declared = packages.iter().any(|package| {
        package.targets.iter().any(|candidate| {
            &candidate.name == bin && candidate.kind.iter().any(|kind| kind == BIN_KIND)
        })
    });
    if declared {
        return Ok(());
    }
    Err(MechanismError::UnknownBin {
        target: target.to_owned(),
        output: output.to_owned(),
        bin: preview(bin),
        candidates: names(packages.iter().flat_map(|package| {
            package
                .targets
                .iter()
                .filter(|candidate| candidate.kind.iter().any(|kind| kind == BIN_KIND))
                .map(|candidate| candidate.name.clone())
        })),
    })
}

/// A bounded, deterministic candidate list for a refusal.
fn names(values: impl Iterator<Item = String>) -> String {
    let mut listed: Vec<String> = values.collect();
    listed.sort_unstable();
    listed.dedup();
    if listed.is_empty() {
        return "none".to_owned();
    }
    let kept = listed.len().min(AMBIGUITY_PREVIEW);
    let head = listed[..kept].join(", ");
    if kept == listed.len() {
        head
    } else {
        format!("{head}, and {} more", listed.len() - kept)
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod tests;
