//! The artifact-plan validation laws and the one refusal family they raise.
//!
//! Split out of the artifact cell along the seam between what an input IS
//! (identity, kinds, the carried subject) and what makes one LAWFUL: the
//! origin/target package relation, the scalar text law, the declared-path
//! separator law, and the alias-machinery probe. Every caller reaches them
//! through the artifact module's re-exports, so the split moved no path.

use crate::Directives;

use super::super::DocumentSubject;
use super::{ArtifactTarget, SpecAddress};

/// Crate-private so the wire conversion can re-run the same origin/target law
/// on every normal/hoisted contribution a carrier carries, at every level.
pub(crate) fn validate_package_relation(
    kind: &'static str,
    origin: &str,
    target: &SpecAddress,
    whole_unversioned: bool,
) -> Result<(), ArtifactPlanError> {
    let coordinate = origin.split_whitespace().next().unwrap_or_default();
    let suffix = origin.strip_prefix(coordinate).unwrap_or_default();
    let suffix_valid =
        suffix.is_empty() || (suffix.starts_with(" [shared by ") && suffix.ends_with(']'));
    if !suffix_valid {
        return Err(identity_error(
            kind,
            origin,
            target,
            "origin carries an unsupported suffix",
        ));
    }
    let Some((origin_group, origin_name)) = coordinate.split_once('/') else {
        return Err(identity_error(
            kind,
            origin,
            target,
            "origin is not a package coordinate",
        ));
    };
    let crate::Authority::Package {
        group,
        name,
        version,
    } = &target.authority
    else {
        return Err(identity_error(
            kind,
            origin,
            target,
            "target uses host authority",
        ));
    };
    if group != origin_group || name != origin_name {
        return Err(identity_error(
            kind,
            origin,
            target,
            "origin and target package coordinates differ",
        ));
    }
    if whole_unversioned
        && (version.is_some() || !target.anchor.is_empty() || target.pinned_r.is_some())
    {
        return Err(identity_error(
            kind,
            origin,
            target,
            "hoisted target must be an unversioned whole document",
        ));
    }
    Ok(())
}

fn identity_error(
    kind: &'static str,
    origin: &str,
    target: &SpecAddress,
    reason: &'static str,
) -> ArtifactPlanError {
    ArtifactPlanError::InputIdentity {
        kind,
        origin: origin.to_string(),
        target: target.to_string(),
        reason,
    }
}

pub(super) fn validate_text(field: &'static str, value: &str) -> Result<(), ArtifactPlanError> {
    if value.trim().is_empty() {
        return Err(ArtifactPlanError::Blank { field });
    }
    if value.contains(['\n', '\r', '\0']) {
        return Err(ArtifactPlanError::UnsafeText { field });
    }
    Ok(())
}

/// The separator law on a path that becomes a [`DocumentSubject`]'s
/// `declared_path`.
///
/// Applied to the contribution path specifically, never through
/// [`validate_text`]: `validate_text` also judges origins, artifact ids and
/// the static-lane roots, and widening it would hold values that are not
/// selector paths to a selector path's contract. The rule itself lives once,
/// on [`DocumentSubject`]; this is the artifact plan's vocabulary for it.
pub(super) fn validate_declared_path(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPlanError> {
    if DocumentSubject::path_is_forward_slashed(value) {
        return Ok(());
    }
    Err(ArtifactPlanError::BackslashedPath {
        field,
        value: value.to_string(),
    })
}

pub(super) fn simple_alias_machinery(text: &str) -> bool {
    let directives = Directives::parse(text);
    !directives.aliases.is_empty()
        || directives
            .errors
            .iter()
            .any(|error| error.message.contains("undeclared alias"))
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactPlanError {
    #[error("compiler {field} must not be blank")]
    Blank { field: &'static str },
    #[error("compiler {field} must not contain a newline or NUL")]
    UnsafeText { field: &'static str },
    #[error(
        "compiler {field} `{value}` must be forward-slashed: a `paths` selector dimension compiles its globs with a literal separator, so a backslashed path matches nothing at all"
    )]
    BackslashedPath { field: &'static str, value: String },
    #[error("artifact input {index} is invalid: {source}")]
    Input {
        index: usize,
        #[source]
        source: Box<ArtifactPlanError>,
    },
    #[error("simple input identity must be {expected:?}, got {actual:?}")]
    SimpleIdentity { expected: String, actual: String },
    #[error("simple input subject must be {expected}, got {actual}")]
    SimpleSubject { expected: String, actual: String },
    #[error(
        "invalid artifact context tuple: id `{artifact}`, target {target:?}, frame {frame}, mode {mode}"
    )]
    InvalidContextTuple {
        artifact: String,
        target: ArtifactTarget,
        frame: String,
        mode: String,
    },
    #[error("simple artifact inputs must use canonical Markdown, got `{actual}`")]
    SimpleFormat { actual: String },
    #[error(
        "the simple package `{origin}` carries alias machinery (`#use … as` / `@!`) that is `normal`-format only (PROP-035 §7.2); convert the package to `format = \"normal\"` or drop the alias"
    )]
    SimpleAlias { index: usize, origin: String },
    #[error(
        "simple artifact inputs {first} and {second} claim `{origin}:{path}` with different source text"
    )]
    ConflictingSimpleIdentity {
        first: usize,
        second: usize,
        origin: String,
        path: String,
    },
    #[error("{kind} input `{origin}` contradicts target `{target}`: {reason}")]
    InputIdentity {
        kind: &'static str,
        origin: String,
        target: String,
        reason: &'static str,
    },
}
