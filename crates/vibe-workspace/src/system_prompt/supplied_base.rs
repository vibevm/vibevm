//! Reading a harness's own base prompt from versioned data with provenance
//! (PROP-076 `#SYS-SUPPLIED-BASE`).
//!
//! **Why this cell exists.** A harness whose only subagent-safe instruction
//! slot *replaces* its base prompt cannot be given a boot lane unless vibe
//! supplies that base itself — OpenCode's per-agent `system` and Qwen Code's
//! `QWEN_SYSTEM_MD` are both of that shape. The base is therefore data: it
//! ships in the harness's own vibe package, per harness version, beside a
//! `PROVENANCE.toml` that records where every byte came from.
//!
//! **This file is an INTERPRETER, and that is the whole design rule.** No
//! prompt text, no token spelling, no bullet string and no model-matching
//! substring may appear here: every one of them is read from the provenance
//! document. A harness version whose composition needs a rule this
//! interpreter cannot express is **refused** — [`Provenance::compose`]
//! answers an error, the caller soft-launches, and a human extends either the
//! document or this interpreter deliberately. A special case in Rust for one
//! harness version would be a literal wearing a different hat, and it would
//! put us back where `#SYS-SUPPLIED-BASE` started.
//!
//! **A wrong base is worse than no lane.** A base from a stale version, or
//! for the wrong model family, produces a session whose harness instructions
//! are not the ones that harness wrote, with no error anywhere. So every
//! answer here is either exact or an error: digests are verified before the
//! bytes are used, `verified = false` refuses, and there is deliberately no
//! nearest-version fallback — a base for 2.2.6 is not a base for 2.2.7.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SUPPLIED-BASE");

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The schema version this interpreter understands. A document declaring
/// another one is refused rather than read optimistically: the whole point of
/// the genre is that nothing is guessed.
pub const PROVENANCE_SCHEMA: u32 = 1;

/// The file that describes one harness version's bases.
pub const PROVENANCE_FILE: &str = "PROVENANCE.toml";

/// The directory in a harness package that holds the per-version sets.
pub const PROMPT_BASES_DIR: &str = "prompt-bases";

/// Why a base could not be supplied. Every variant is a sentence a soft
/// launch prints, because each names a different thing the operator can fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseError {
    /// No set for this harness version. Named rather than approximated:
    /// there is no nearest-version fallback by design.
    NoSet {
        harness: String,
        version: String,
        looked_in: PathBuf,
    },
    /// The document is there and unreadable, or not valid TOML.
    Unreadable { path: PathBuf, reason: String },
    /// A schema this build does not interpret.
    Schema { found: u32, understood: u32 },
    /// The document describes another harness or another version than the one
    /// asked for — a mis-filed directory, which must not be used silently.
    Mismatch {
        wanted: String,
        found: String,
        field: &'static str,
    },
    /// The proof has not been run for this set, so no launch may deliver it.
    NotVerified { harness: String, version: String },
    /// A recorded file is missing from the set.
    MissingFile { file: String },
    /// A recorded digest does not match the bytes on disk — the one failure
    /// that means the data itself is not what it claims.
    DigestMismatch {
        file: String,
        expected: String,
        found: String,
    },
    /// The launch's model is not knowable, so no selection row can be
    /// applied and the default would be a guess for a family that has its
    /// own base.
    UnknownModel,
    /// A rule the document states and this interpreter cannot express.
    Unsupported { what: String },
}

impl fmt::Display for BaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaseError::NoSet {
                harness,
                version,
                looked_in,
            } => write!(
                f,
                "no prompt-base set for {harness} {version}: `{}` is not there. The set ships \
                 with the harness's own vibe package, one directory per harness version, and \
                 there is deliberately no fallback to another version's base",
                looked_in.display()
            ),
            BaseError::Unreadable { path, reason } => {
                write!(f, "`{}` could not be read ({reason})", path.display())
            }
            BaseError::Schema { found, understood } => write!(
                f,
                "the prompt-base provenance declares schema {found} and this build interprets \
                 {understood}"
            ),
            BaseError::Mismatch {
                wanted,
                found,
                field,
            } => write!(
                f,
                "the prompt-base provenance is for `{field}` {found}, not {wanted} — a mis-filed \
                 set is never used on the chance that it fits"
            ),
            BaseError::NotVerified { harness, version } => write!(
                f,
                "the prompt-base set for {harness} {version} is not verified: `verified` is \
                 false, so the base it carries has not been measured against the installed \
                 program and may not be delivered"
            ),
            BaseError::MissingFile { file } => {
                write!(
                    f,
                    "the prompt-base set records `{file}`, which is not in it"
                )
            }
            BaseError::DigestMismatch {
                file,
                expected,
                found,
            } => write!(
                f,
                "`{file}` in the prompt-base set does not match its recorded digest (expected \
                 {expected}, found {found}) — the data is not what its provenance claims"
            ),
            BaseError::UnknownModel => write!(
                f,
                "this launch's model could not be determined, and the base a harness uses \
                 depends on it, so no base may be chosen"
            ),
            BaseError::Unsupported { what } => write!(
                f,
                "the prompt-base provenance states a rule this build cannot express ({what}); \
                 it is refused rather than approximated"
            ),
        }
    }
}

/// One base file as the provenance records it.
#[derive(Debug, Clone, Deserialize)]
pub struct Base {
    /// The id selection rows refer to.
    pub id: String,
    /// Its name inside the set's directory.
    pub file: String,
    /// Where it came from upstream, for a reader rather than for the code.
    #[serde(default)]
    pub upstream_path: String,
    /// The length its bytes must have.
    pub bytes: usize,
    /// The digest its bytes must have.
    pub sha256: String,
    /// Whether this file carries the render token at all. Recorded because a
    /// base without it receives no rendered text, which is a visible
    /// behavioural difference rather than an optimisation.
    #[serde(default)]
    pub has_tool_guidance_token: bool,
    /// The base used when no selection row matches.
    #[serde(default)]
    pub default: bool,
}

/// A literal substitution a selection row applies to its base.
#[derive(Debug, Clone, Deserialize)]
pub struct Substitute {
    /// The literal token to replace.
    pub token: String,
    /// Which launch value replaces it. A name this interpreter knows, so an
    /// unknown one is a refusal rather than an empty string.
    pub value: String,
    /// `all` or `first`.
    #[serde(default = "all_occurrences")]
    pub occurrences: String,
}

fn all_occurrences() -> String {
    "all".to_owned()
}

/// How a base replaces or extends the default one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// This base replaces the default base.
    Override,
    /// The default base is kept and this one is appended behind a blank line.
    Append,
}

/// One row of the model→base table. Rows are tried in order and the first
/// whose every substring is present wins, which is how a recorded order
/// reproduces an upstream conditional without a special case in code.
#[derive(Debug, Clone, Deserialize)]
pub struct Select {
    /// Every one of these must appear in the lower-cased model id.
    pub substrings: Vec<String>,
    /// The [`Base::id`] this row selects.
    pub base: String,
    /// `override` or `append`.
    pub mode: String,
    /// Where the row was read from, for a human reading the record.
    #[serde(default)]
    pub source: String,
    /// Literal substitutions this row's base needs.
    #[serde(default)]
    pub substitute: Vec<Substitute>,
}

/// One gated line of the rendered guidance.
#[derive(Debug, Clone, Deserialize)]
pub struct Bullet {
    /// The tool whose presence in the launch's tool list adds this line.
    pub gate: String,
    /// The line itself.
    pub text: String,
}

/// How the harness expands its tool-guidance token.
#[derive(Debug, Clone, Deserialize)]
pub struct Render {
    /// The literal token.
    pub token: String,
    /// `first` or `all`.
    #[serde(default = "first_occurrence")]
    pub replace: String,
    /// What the kept bullets are joined with.
    #[serde(default = "newline")]
    pub join: String,
    /// Where the rule was read from.
    #[serde(default)]
    pub source: String,
    /// The bullets, in the order the harness emits them — which is the
    /// order written here, not the order of the tool list.
    #[serde(default)]
    pub bullet: Vec<Bullet>,
}

fn first_occurrence() -> String {
    "first".to_owned()
}

fn newline() -> String {
    "\n".to_owned()
}

/// How the lane is handed over, as the document states it.
#[derive(Debug, Clone, Deserialize)]
pub struct Carrier {
    /// The mechanism's name. An unknown one is refused by the caller.
    pub kind: String,
    /// The environment variable the configuration travels in.
    #[serde(default)]
    pub env: String,
    /// Flags the launch must pass for the carrier to apply.
    #[serde(default)]
    pub flags: Vec<String>,
    /// The token in the composed text that stands for the lane's path.
    #[serde(default)]
    pub lane_token: String,
    /// What the harness does to the inserted lane: `none` or `trim`.
    #[serde(default)]
    pub lane_transform: String,
    /// What goes between the base's last byte and the lane's first.
    #[serde(default)]
    pub separator_between_base_and_lane: String,
}

/// What has been proved about this set.
#[derive(Debug, Clone, Deserialize)]
pub struct Verification {
    /// True only when the whole-request proof has been run for this exact
    /// harness version. Nothing but that proof may set it.
    #[serde(default)]
    pub verified: bool,
    /// The proof's identifier, for the record.
    #[serde(default)]
    pub proof_id: String,
    /// When it was run.
    #[serde(default)]
    pub proof_date: String,
}

/// One harness version's prompt-base set.
#[derive(Debug, Clone, Deserialize)]
pub struct Provenance {
    /// The document schema.
    pub schema: u32,
    /// Which harness this describes.
    pub harness: String,
    /// Which version of it.
    pub harness_version: String,
    /// The base files.
    #[serde(default)]
    pub base: Vec<Base>,
    /// The model→base table, in order.
    #[serde(default)]
    pub select: Vec<Select>,
    /// The token-expansion rule.
    pub render: Option<Render>,
    /// The hand-over mechanism.
    pub carrier: Option<Carrier>,
    /// What has been proved.
    #[serde(default)]
    pub verification: Option<Verification>,
}

/// The facts of one launch that the recorded rules are applied to.
#[derive(Debug, Clone)]
pub struct Launch<'a> {
    /// The model id, as the harness would see it. Matching is a substring
    /// test on its lower-cased form.
    pub model_id: &'a str,
    /// The model's display name, for a `model_name` substitution.
    pub model_name: &'a str,
    /// The tool names this launch will offer, which gate the render bullets.
    pub tools: &'a [String],
}

impl Provenance {
    /// Read one set's document. The bytes are not trusted yet — [`compose`]
    /// verifies every digest before using a base.
    ///
    /// [`compose`]: Provenance::compose
    pub fn read(dir: &Path) -> Result<Self, BaseError> {
        let path = dir.join(PROVENANCE_FILE);
        let text = std::fs::read_to_string(&path).map_err(|error| BaseError::Unreadable {
            path: path.clone(),
            reason: error.to_string(),
        })?;
        let parsed: Provenance = toml::from_str(&text).map_err(|error| BaseError::Unreadable {
            path,
            reason: error.to_string(),
        })?;
        if parsed.schema != PROVENANCE_SCHEMA {
            return Err(BaseError::Schema {
                found: parsed.schema,
                understood: PROVENANCE_SCHEMA,
            });
        }
        Ok(parsed)
    }

    /// The directory a harness package's installed source keeps one
    /// version's set in.
    pub fn set_dir(package_source: &Path, harness_version: &str) -> PathBuf {
        package_source.join(PROMPT_BASES_DIR).join(harness_version)
    }

    /// Which base this launch's model takes, and how it combines with the
    /// default one.
    ///
    /// **Order is the rule.** Rows are tried as written and the first whose
    /// every substring is present wins, so a document can reproduce an
    /// upstream conditional — "astra if the id also says astra, else gpt" —
    /// by writing the narrower row first. No row matching is not an error:
    /// it is the default base, which is what the harness itself does.
    pub fn select_for(&self, model_id: &str) -> Result<Chosen<'_>, BaseError> {
        if model_id.trim().is_empty() {
            return Err(BaseError::UnknownModel);
        }
        let id = model_id.to_ascii_lowercase();
        let default =
            self.base
                .iter()
                .find(|base| base.default)
                .ok_or_else(|| BaseError::Unsupported {
                    what: "no base is marked `default`, and a model that matches no row needs one"
                        .to_owned(),
                })?;
        for row in &self.select {
            if row.substrings.is_empty() {
                return Err(BaseError::Unsupported {
                    what: "a selection row with no `substrings` would match every model".to_owned(),
                });
            }
            if !row.substrings.iter().all(|needle| id.contains(needle)) {
                continue;
            }
            let mode = match row.mode.as_str() {
                "override" => Mode::Override,
                "append" => Mode::Append,
                other => {
                    return Err(BaseError::Unsupported {
                        what: format!("selection mode `{other}`"),
                    });
                }
            };
            let selected = self
                .base
                .iter()
                .find(|base| base.id == row.base)
                .ok_or_else(|| BaseError::Unsupported {
                    what: format!(
                        "selection row names base `{}`, which is not recorded",
                        row.base
                    ),
                })?;
            return Ok(Chosen {
                default,
                selected: Some(selected),
                mode,
                substitute: &row.substitute,
            });
        }
        Ok(Chosen {
            default,
            selected: None,
            mode: Mode::Override,
            substitute: &[],
        })
    }

    /// Expand the recorded token in `template` from the launch's tool list.
    ///
    /// A template without the token is returned unchanged, which is not an
    /// optimisation: the document records `has_tool_guidance_token` per base
    /// precisely because some families receive no guidance at all, and that
    /// is a behavioural fact worth being able to read.
    pub fn render_guidance(&self, template: &str, tools: &[String]) -> Result<String, BaseError> {
        let Some(render) = &self.render else {
            return Ok(template.to_owned());
        };
        let kept: Vec<&str> = render
            .bullet
            .iter()
            .filter(|bullet| tools.iter().any(|tool| tool == &bullet.gate))
            .map(|bullet| bullet.text.as_str())
            .collect();
        let expansion = kept.join(&render.join);
        Ok(match render.replace.as_str() {
            "first" => template.replacen(&render.token, &expansion, 1),
            "all" => template.replace(&render.token, &expansion),
            other => {
                return Err(BaseError::Unsupported {
                    what: format!("render replacement `{other}`"),
                });
            }
        })
    }

    /// The harness's own base prompt for this launch, composed from the set
    /// in `dir`.
    ///
    /// Every digest is verified before its bytes are used, and an unverified
    /// set refuses outright: those two checks are what separate "the base we
    /// recorded" from "a base we hope is right".
    pub fn compose(&self, dir: &Path, launch: &Launch<'_>) -> Result<String, BaseError> {
        if !self
            .verification
            .as_ref()
            .is_some_and(|verification| verification.verified)
        {
            return Err(BaseError::NotVerified {
                harness: self.harness.clone(),
                version: self.harness_version.clone(),
            });
        }
        let chosen = self.select_for(launch.model_id)?;
        let mut text = match (chosen.selected, chosen.mode) {
            // The default base alone.
            (None, _) => {
                self.render_guidance(&self.read_base(dir, chosen.default)?, launch.tools)?
            }
            // This family's own base instead of the default one.
            (Some(base), Mode::Override) => {
                self.render_guidance(&self.read_base(dir, base)?, launch.tools)?
            }
            // The default base, then this family's addendum behind a blank
            // line — the one combining form a harness has been seen to use.
            (Some(base), Mode::Append) => {
                let head =
                    self.render_guidance(&self.read_base(dir, chosen.default)?, launch.tools)?;
                let tail = self.render_guidance(&self.read_base(dir, base)?, launch.tools)?;
                format!("{}\n\n{}", head.trim_end_matches('\n'), tail)
            }
        };
        for substitution in chosen.substitute {
            let value = match substitution.value.as_str() {
                "model_name" => launch.model_name,
                "model_id" => launch.model_id,
                other => {
                    return Err(BaseError::Unsupported {
                        what: format!("substitution value `{other}`"),
                    });
                }
            };
            text = match substitution.occurrences.as_str() {
                "all" => text.replace(&substitution.token, value),
                "first" => text.replacen(&substitution.token, value, 1),
                other => {
                    return Err(BaseError::Unsupported {
                        what: format!("substitution occurrences `{other}`"),
                    });
                }
            };
        }
        Ok(text)
    }

    /// Read one base's bytes and hold them to their record.
    fn read_base(&self, dir: &Path, base: &Base) -> Result<String, BaseError> {
        let path = dir.join(&base.file);
        let bytes = std::fs::read(&path).map_err(|_| BaseError::MissingFile {
            file: base.file.clone(),
        })?;
        if bytes.len() != base.bytes {
            return Err(BaseError::DigestMismatch {
                file: base.file.clone(),
                expected: format!("{} bytes", base.bytes),
                found: format!("{} bytes", bytes.len()),
            });
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if !digest.eq_ignore_ascii_case(&base.sha256) {
            return Err(BaseError::DigestMismatch {
                file: base.file.clone(),
                expected: base.sha256.clone(),
                found: digest,
            });
        }
        String::from_utf8(bytes).map_err(|_| BaseError::Unreadable {
            path,
            reason: "not UTF-8".to_owned(),
        })
    }

    /// Verify every recorded file in the set, including the ones this launch
    /// does not use. The sync tool and `vibe prompt check` want the whole
    /// set's integrity, not only the part one model reaches.
    pub fn verify_all(&self, dir: &Path) -> Result<BTreeMap<String, usize>, BaseError> {
        let mut sizes = BTreeMap::new();
        for base in &self.base {
            let text = self.read_base(dir, base)?;
            sizes.insert(base.file.clone(), text.len());
        }
        Ok(sizes)
    }
}

/// Which base a model takes, and how it combines with the default one.
#[derive(Debug, Clone)]
pub struct Chosen<'a> {
    /// The base used when no row matches, and the head of an `append`.
    pub default: &'a Base,
    /// The row's own base, when a row matched.
    pub selected: Option<&'a Base>,
    /// How the two combine.
    pub mode: Mode,
    /// Literal substitutions the row asks for.
    pub substitute: &'a [Substitute],
}

#[cfg(test)]
#[path = "supplied_base/tests.rs"]
mod tests;
