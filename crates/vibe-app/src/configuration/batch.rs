//! Classify shared answers at every actual selected installer before app effects.
use super::{ConfigurationError, ConfigurationSubmission};
use std::collections::BTreeSet;
use vibe_core::manifest::InstallerOptionDecl;

/// A public answer in one root must never become another root's secret implicitly.
/// Roots are actual selected installer options, not package-name guesses.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-JOURNEY"
)]
pub fn classify_shared(
    roots: &[(&str, &InstallerOptionDecl)],
    set: &[(String, String)],
    submission: Option<&ConfigurationSubmission>,
) -> Result<(), ConfigurationError> {
    let ids: BTreeSet<_> = set
        .iter()
        .map(|(id, _)| id.as_str())
        .chain(
            submission
                .into_iter()
                .flat_map(|s| s.input.intent.parameters.keys().map(String::as_str)),
        )
        .collect();
    for (key, option) in roots {
        if submission.is_some() && option.configuration.is_none() {
            return Err(ConfigurationError::Invalid {
                option: format!("{key}/{}", option.id),
                reason: "a complete common input cannot be applied to a legacy installer".into(),
            });
        }
    }
    for id in ids {
        let mut public = Vec::new();
        let mut secret = Vec::new();
        let mut missing = Vec::new();
        for (key, option) in roots {
            let declarations = option
                .configuration
                .as_ref()
                .map_or(option.parameters.as_slice(), |c| c.parameters.as_slice());
            match declarations.iter().find(|p| p.id == id) {
                Some(p) if p.secret => secret.push(*key),
                Some(_) => public.push(*key),
                None => missing.push(*key),
            }
        }
        let reason = if !public.is_empty() && !secret.is_empty() {
            Some(format!(
                "parameter `{id}` is public for [{}] and secret for [{}]; submit separate per-root requests",
                public.join(", "),
                secret.join(", ")
            ))
        } else if !missing.is_empty() {
            Some(format!(
                "parameter `{id}` is not declared by [{}]; submit separate per-root requests",
                missing.join(", ")
            ))
        } else if !secret.is_empty()
            && submission.is_some_and(|s| s.input.intent.parameters.contains_key(id))
        {
            Some(format!(
                "parameter `{id}` is secret for [{}]; use ephemeral per-root input",
                secret.join(", ")
            ))
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(ConfigurationError::Invalid {
                option: "shared batch input".into(),
                reason,
            });
        }
    }
    Ok(())
}
