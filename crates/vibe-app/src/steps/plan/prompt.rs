//! Prepare only running agent prompts and retain the checked literal source.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#SECRET-TEMPLATES");

use crate::error::{AppError, StepError};
use crate::steps::template::{self, Vars};
use std::path::Path;
use vibe_core::manifest::{InstallerOptionDecl, StepDecl, StepKind, declarant_path};

pub(super) fn prepare(
    option: &InstallerOptionDecl,
    step: &StepDecl,
    run: bool,
    root: &Path,
    vars: &Vars<'_>,
) -> Result<Option<String>, AppError> {
    if !run {
        return Ok(None);
    }
    let StepKind::Agent {
        prompt,
        prompt_file,
        ..
    } = &step.kind
    else {
        return Ok(None);
    };
    let error = |field, reason| {
        AppError::from(StepError::Template {
            step: step.id.clone(),
            field,
            reason,
        })
    };
    let text = match (prompt, prompt_file) {
        (Some(text), None) => text.clone(),
        (None, Some(file)) => {
            let relative = declarant_path(file)
                .map_err(|fault| error("prompt_file", fault.reason().into()))?;
            // Only the explicit caller-owned root is canonicalized. Descendants
            // are traversed by the shared-inode no-follow source observer.
            let root = root.canonicalize().map_err(|e| {
                error(
                    "prompt_file",
                    format!("package root cannot be admitted: {e}"),
                )
            })?;
            let source = vibe_safefs::Project::open(&root).map_err(|e| {
                error(
                    "prompt_file",
                    format!("package root cannot be opened: {e:#}"),
                )
            })?;
            let bytes = source
                .read_source_bytes(relative)
                .map_err(|e| error("prompt_file", format!("`{relative}` cannot be read: {e:#}")))?
                .ok_or_else(|| error("prompt_file", format!("`{relative}` does not exist")))?;
            String::from_utf8(bytes)
                .map_err(|_| error("prompt_file", format!("`{relative}` is not UTF-8")))?
        }
        _ => {
            return Err(error(
                "prompt",
                "exactly one prompt or prompt_file is required".into(),
            ));
        }
    };
    option
        .validate_prompt_template(&text)
        .map_err(|reason| error("prompt", reason))?;
    template::preview(&text, vars, &step.id, step.phase)?;
    Ok(Some(text))
}

#[cfg(test)]
mod tests;
