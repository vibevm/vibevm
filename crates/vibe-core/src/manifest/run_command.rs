//! Explicit host-owned commands dispatched by `vibe run`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-RUN-COMMANDS");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::{declarant_path, is_portable_token};

/// One explicit command in a project/workspace manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#HOST-RUN-COMMANDS")]
pub struct RunCommandDecl {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub handler: RunCommandHandler,
}

/// Closed v1 command-handler vocabulary. A shell command string is
/// deliberately not a handler: the manifest names a portable script base and
/// Vibe selects the platform wrapper without asking a shell to parse text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#HOST-RUN-COMMANDS")]
pub enum RunCommandHandler {
    Script { base: PathBuf },
}

impl RunCommandDecl {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err("[[command]].id must be a portable lowercase token".into());
        }
        if self
            .description
            .as_deref()
            .is_some_and(|description| description.trim().is_empty())
        {
            return Err("[[command]].description must not be blank".into());
        }
        match &self.handler {
            RunCommandHandler::Script { base } => {
                declarant_path(base)
                    .map(|_| ())
                    .map_err(|fault| {
                        format!(
                            "[[command]] `{}` handler base must be a portable project-relative path: {}",
                            self.id,
                            fault.reason()
                        )
                    })
            }
        }
    }
}
