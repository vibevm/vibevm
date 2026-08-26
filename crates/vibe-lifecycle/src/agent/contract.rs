//! The declared output contract — the first gate an agent execution passes.
//!
//! `config.outputs` is free TOML in the manifest, so its shape is asserted
//! here rather than assumed: a non-empty ordered array of exact rows
//! `{ path, kind = "file", accept = "non-empty file" }`. Every refusal in this
//! cell happens at plan time, strictly before a credential is read or a
//! provider is called — an unknown acceptance predicate that only surfaced
//! after the tokens were spent would be indistinguishable from a passing one.
//!
//! Component legality is **not** decided here: it is delegated to the one
//! shared portable-name law in `vibe_safefs`, which also owns the Windows
//! device table and this crate's reserved staging prefix. What this cell adds
//! on top is the part only a *set* of declared outputs can have — two rows
//! that name the same physical file, or one row that must be both a file and
//! another row's directory.

use serde_json::Value;
use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::Context;
use vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact;
use vibe_wire::generated::lifecycle_state::StateArtifact;

use super::AgentError;

/// The one artifact shape epoch-1 supports.
pub const OUTPUT_KIND_FILE: &str = "file";
/// The one acceptance predicate epoch-1 supports.
pub const OUTPUT_ACCEPT_NON_EMPTY: &str = "non-empty file";

/// One declared output row, already checked for containment safety.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct OutputRow {
    path: String,
    /// The case-folded component vector — this row's physical identity as far
    /// as a case-insensitive filesystem is concerned.
    key: Vec<String>,
}

impl OutputRow {
    /// The project-relative, forward-slashed path exactly as declared.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// The complete ordered contract for one agent execution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct OutputContract {
    rows: Vec<OutputRow>,
}

impl OutputContract {
    /// Read and check `config.outputs` from one already-built envelope.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
    pub fn parse(context: &Context) -> Result<Self, AgentError> {
        let declared = context
            .execution
            .config
            .get("outputs")
            .and_then(Option::as_ref)
            .ok_or_else(|| AgentError::Contract {
                reason: "`config.outputs` is absent".into(),
            })?;
        let Value::Array(declared) = declared else {
            return Err(AgentError::Contract {
                reason: "`config.outputs` is not an array".into(),
            });
        };
        if declared.is_empty() {
            return Err(AgentError::Contract {
                reason: "`config.outputs` is empty; an agent execution with nothing to produce is \
                         a declaration mistake, not a no-op"
                    .into(),
            });
        }
        let mut rows: Vec<OutputRow> = Vec::with_capacity(declared.len());
        for (index, row) in declared.iter().enumerate() {
            let row = parse_row(index, row)?;
            for prior in &rows {
                check_pair(index, prior, &row)?;
            }
            rows.push(row);
        }
        Ok(Self { rows })
    }

    /// The declared rows in declaration order — the order the provider result
    /// must reproduce exactly.
    #[must_use]
    pub fn rows(&self) -> &[OutputRow] {
        &self.rows
    }

    /// The declared paths, for state/artifact comparison.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.rows.iter().map(|row| row.path.clone()).collect()
    }

    /// The exact artifact rows this contract will produce, derived from the
    /// declaration alone. They are the *plan*: the generic row law judges them
    /// before a token is spent, the provider may supply only their content,
    /// and freshness authenticates a recorded set by comparing against them.
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn planned_rows(&self, project_root: &str) -> Vec<ReplyArtifact> {
        let root = project_root.trim_end_matches('/');
        self.rows
            .iter()
            .map(|row| ReplyArtifact {
                id: row.path.clone(),
                kind: OUTPUT_KIND_FILE.to_string(),
                path: format!("{root}/{}", row.path),
            })
            .collect()
    }

    /// The same rows in the durable state shape, for exact freshness equality.
    #[must_use]
    pub fn planned_state_rows(&self, project_root: &str) -> Vec<StateArtifact> {
        self.planned_rows(project_root)
            .into_iter()
            .map(|row| StateArtifact {
                id: row.id,
                kind: row.kind,
                path: row.path,
            })
            .collect()
    }

    /// The exact prose the prompt publishes, so the provider sees the same
    /// contract the handler will enforce.
    #[must_use]
    pub fn prose(&self) -> String {
        let mut prose = String::new();
        for (index, row) in self.rows.iter().enumerate() {
            prose.push_str(&format!(
                "{}. path = \"{}\", kind = \"{OUTPUT_KIND_FILE}\", \
                 accept = \"{OUTPUT_ACCEPT_NON_EMPTY}\"\n",
                index + 1,
                row.path,
            ));
        }
        prose
    }
}

/// Two declared rows may not name the same physical file, and one may not have
/// to be a directory for another while also being a file itself.
fn check_pair(index: usize, prior: &OutputRow, row: &OutputRow) -> Result<(), AgentError> {
    let refuse = |what: &str| {
        Err(AgentError::Contract {
            reason: format!(
                "`config.outputs[{index}]` (`{}`) {what} `{}`; each declared output is one \
                 distinct file written exactly once",
                row.path, prior.path,
            ),
        })
    };
    if row.key == prior.key {
        return if row.path == prior.path {
            refuse("repeats")
        } else {
            // `Docs/a.md` and `docs/a.md` are two spellings of one file on
            // Windows and on case-insensitive APFS: the second rename would
            // silently replace the first while the reply claimed two writes.
            refuse("is a case-folded alias of")
        };
    }
    if row.key.starts_with(prior.key.as_slice()) || prior.key.starts_with(row.key.as_slice()) {
        // `docs` and `docs/a.md`: one row must be a regular file, the other
        // needs it to be a directory. Refused before the paid call, not at the
        // write, where half the contract would already be on disk.
        return refuse("overlaps the path prefix of");
    }
    Ok(())
}

fn parse_row(index: usize, row: &Value) -> Result<OutputRow, AgentError> {
    let Value::Object(row) = row else {
        return Err(AgentError::Contract {
            reason: format!("`config.outputs[{index}]` is not a table"),
        });
    };
    let mut unknown: Vec<&str> = row
        .keys()
        .map(String::as_str)
        .filter(|key| !matches!(*key, "path" | "kind" | "accept"))
        .collect();
    unknown.sort_unstable();
    if !unknown.is_empty() {
        return Err(AgentError::Contract {
            reason: format!(
                "`config.outputs[{index}]` carries unknown key(s) {unknown:?}; epoch-1 rows are \
                 exactly {{ path, kind, accept }}"
            ),
        });
    }
    let field = |name: &str| -> Result<String, AgentError> {
        match row.get(name) {
            Some(Value::String(value)) => Ok(value.clone()),
            Some(_) => Err(AgentError::Contract {
                reason: format!("`config.outputs[{index}].{name}` is not a string"),
            }),
            None => Err(AgentError::Contract {
                reason: format!("`config.outputs[{index}].{name}` is absent"),
            }),
        }
    };
    let kind = field("kind")?;
    if kind != OUTPUT_KIND_FILE {
        return Err(AgentError::Contract {
            reason: format!(
                "`config.outputs[{index}].kind` is `{kind}`; this epoch supports exactly \
                 `{OUTPUT_KIND_FILE}`"
            ),
        });
    }
    let accept = field("accept")?;
    if accept != OUTPUT_ACCEPT_NON_EMPTY {
        return Err(AgentError::Contract {
            reason: format!(
                "`config.outputs[{index}].accept` is `{accept}`; this epoch supports exactly \
                 `{OUTPUT_ACCEPT_NON_EMPTY}`"
            ),
        });
    }
    let path = field("path")?;
    let key = check_path(index, &path)?;
    Ok(OutputRow { path, key })
}

/// Project-relative and portable. Rootedness is judged here (a project-
/// relative declaration is this cell's own rule); every component is judged by
/// the shared law, which owns backslashes, colons/ADS, device names, trailing
/// dot/space, control characters and the reserved staging prefix.
fn check_path(index: usize, path: &str) -> Result<Vec<String>, AgentError> {
    let refuse = |reason: String| AgentError::Contract {
        reason: format!("`config.outputs[{index}].path` (`{path}`) {reason}"),
    };
    if path.starts_with('/') {
        return Err(refuse(
            "is rooted; declared paths are project-relative".into(),
        ));
    }
    if std::path::Path::new(path).is_absolute() {
        return Err(refuse(
            "is absolute; declared paths are project-relative".into(),
        ));
    }
    let (parents, name) =
        vibe_safefs::split_relative(path).map_err(|error| refuse(format!("is unsafe: {error}")))?;
    let mut key: Vec<String> = parents
        .iter()
        .map(|component| vibe_safefs::identity_key(component))
        .collect();
    key.push(vibe_safefs::identity_key(&name));
    Ok(key)
}
