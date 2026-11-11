//! Composing one launch's base from a recorded description (PROP-076
//! `#SYS-SUPPLIED-BASE`).
//!
//! **The sibling cell holds the document; this one applies it.**
//! `supplied_base` is the genre — what a `PROVENANCE.toml` may say and what
//! every refusal of it is called. Here is the interpretation: which base a
//! model takes, how the recorded token expands, how the two combine, and the
//! carrier document the result travels in.
//!
//! **Still no harness string.** Every spelling comes from the document, and a
//! rule outside its recorded vocabulary is [`BaseError::Unsupported`] rather
//! than an approximation — because a special case in Rust for one harness
//! version is a literal wearing a different hat, which is where
//! `#SYS-SUPPLIED-BASE` started.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SUPPLIED-BASE");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{
    Base, BaseError, CarrierDocument, LANE_PLACEHOLDER, Mode, PROMPT_BASES_DIR, PROVENANCE_FILE,
    PROVENANCE_SCHEMA, Provenance, Substitute,
};

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
        self.compose_unverified(dir, launch)
    }

    /// The same composition **without** the `verified` gate — for the prover
    /// and for nothing else.
    ///
    /// **Why this door exists at all.** `verified = true` is earned by a
    /// whole-request measurement of the composition against the installed
    /// program, so the thing that earns it has to compose a set that is not
    /// yet verified. Were there no such entry the gate would be
    /// unopenable, and the obvious workaround — a sync tool that
    /// reimplements the composition — is strictly worse: it would prove its
    /// own bytes rather than vibe's.
    ///
    /// Every **other** gate still holds. Each digest is verified before its
    /// bytes are used, an unknown model is still a refusal, and a rule the
    /// interpreter cannot express is still refused rather than
    /// approximated. The one thing skipped is the record of a proof that is
    /// in progress.
    ///
    /// No launch path may call this. `compose` is the launch's entry, and
    /// the two are one line apart so that reading either shows which is
    /// which.
    pub fn compose_unverified(&self, dir: &Path, launch: &Launch<'_>) -> Result<String, BaseError> {
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
        // **The one transformation the program applies to its own base**, as
        // the document states it. `none` is the default, so a document that
        // says nothing changes nothing.
        match self.render.as_ref().map_or("none", |render| {
            if render.trim.is_empty() {
                "none"
            } else {
                render.trim.as_str()
            }
        }) {
            "none" => {}
            "end" => text = text.trim_end().to_owned(),
            other => {
                return Err(BaseError::Unsupported {
                    what: format!("base trim `{other}`"),
                });
            }
        }
        Ok(text)
    }

    /// The carrier document this launch hands the harness: one
    /// configuration layer declaring vibe's own primary agent, whose prompt
    /// is the composed base followed by the lane's **path** in the
    /// document's own file-inclusion token.
    ///
    /// **No lane byte is in the answer.** The value this returns carries the
    /// base (1,470 bytes for OpenCode 2.2.7) and a path; the ~200 KB of lane
    /// never leaves the file it is in, which is what makes an environment
    /// variable a legitimate carrier at all (`#SYS-HARNESS-DELIVERY`).
    ///
    /// Every key, the agent's name, its mode and the token are the
    /// **document's**, so this function spells no configuration key of
    /// another program; a document missing one of them is refused rather
    /// than defaulted, because a guessed key name is a launch whose agent
    /// silently does not exist.
    pub fn carrier_document(&self, base: &str, lane: &Path) -> Result<CarrierDocument, BaseError> {
        let carrier = self
            .carrier
            .as_ref()
            .ok_or_else(|| BaseError::Unsupported {
                what: "the provenance records no `[carrier]`, so there is nothing to hand over"
                    .to_owned(),
            })?;
        let required = [
            ("env", carrier.env.as_str()),
            ("agents_key", carrier.agents_key.as_str()),
            ("system_key", carrier.system_key.as_str()),
            ("mode_key", carrier.mode_key.as_str()),
            ("default_agent_key", carrier.default_agent_key.as_str()),
            ("agent_name", carrier.agent_name.as_str()),
            ("agent_mode", carrier.agent_mode.as_str()),
            ("lane_token", carrier.lane_token.as_str()),
        ];
        for (field, value) in required {
            if value.is_empty() {
                return Err(BaseError::Unsupported {
                    what: format!("`[carrier] {field}` is empty, and none of it is guessed here"),
                });
            }
        }
        if !carrier.lane_token.contains(LANE_PLACEHOLDER) {
            return Err(BaseError::Unsupported {
                what: format!(
                    "`[carrier] lane_token` does not contain `{LANE_PLACEHOLDER}`, so there is \
                     nowhere to put the lane's path"
                ),
            });
        }
        let token = carrier
            .lane_token
            .replace(LANE_PLACEHOLDER, &lane.display().to_string());
        let system = format!("{base}{}{token}", carrier.separator_between_base_and_lane);
        let mut agent = serde_json::Map::new();
        agent.insert(carrier.mode_key.clone(), carrier.agent_mode.clone().into());
        agent.insert(carrier.system_key.clone(), system.into());
        let mut agents = serde_json::Map::new();
        agents.insert(carrier.agent_name.clone(), agent.into());
        let mut document = serde_json::Map::new();
        document.insert(
            carrier.default_agent_key.clone(),
            carrier.agent_name.clone().into(),
        );
        document.insert(carrier.agents_key.clone(), agents.into());
        Ok(CarrierDocument {
            env: carrier.env.clone(),
            value: serde_json::to_string(&document).map_err(|error| BaseError::Unsupported {
                what: format!("the carrier document could not be serialised ({error})"),
            })?,
            flags: carrier.flags.clone(),
            agent_name: carrier.agent_name.clone(),
        })
    }

    /// The gate tool names this document says a launch through its carrier
    /// offers. Empty when no carrier is recorded, which renders no bullets.
    pub fn gate_tools(&self) -> Vec<String> {
        self.carrier
            .as_ref()
            .map(|carrier| carrier.gate_tools.clone())
            .unwrap_or_default()
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
