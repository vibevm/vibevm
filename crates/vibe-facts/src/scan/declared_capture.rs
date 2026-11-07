//! Held exact-file source observations for the incumbent facts API.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-OBSERVATION-LAW");
use super::declared::{
    CapturedDeclaredDocument, DeclaredDocumentObservation, exact_declared_files,
    observe_declared_source,
};
use super::*;
use vibe_spec::source_policy::PackagePolicyObservation;

pub(super) fn observe(
    root: &Path,
    coordinate: &str,
    policy: &PackagePolicyObservation,
) -> Result<AuthoredSourceObservation, RegistryError> {
    if policy
        .manifest()
        .is_some_and(|manifest| manifest.project.is_some())
    {
        if policy.mode() == vibe_spec::source_policy::PackageSourceMode::ExplicitEmpty {
            policy.recheck_unchanged()?;
            return Ok(AuthoredSourceObservation::Available {
                facts: Vec::new(),
                documents: Vec::new(),
            });
        }
        return Err(policy.unsupported("host facts census").into());
    }
    let package = policy
        .package()
        .ok_or_else(|| policy.unsupported("declared facts without package context"))?;
    if format!("{}/{}", package.group, package.name) != coordinate {
        return Err(RegistryError::InvalidCoordinate {
            coordinate: coordinate.into(),
            reason: "source manifest differs from the selected coordinate".into(),
        });
    }
    let paths =
        exact_declared_files(package).map_err(|_| policy.unsupported("facts directory census"))?;
    let mut held = Vec::new();
    for path in paths {
        let bytes = policy.read_source_bytes(&path)?;
        held.push((path, bytes));
    }
    let documents = held
        .iter()
        .filter_map(|(path, bytes)| bytes.as_deref().map(|b| SourceFileWitness::of(path, b)))
        .collect();
    let inputs: Vec<_> = held
        .iter()
        .map(|(path, bytes)| CapturedDeclaredDocument {
            path,
            bytes: bytes.as_deref(),
        })
        .collect();
    let observed = observe_declared_source(package, &inputs);
    policy.recheck_unchanged()?;
    let invalid = |documents, message| AuthoredSourceObservation::Invalid {
        documents,
        issue: SourceIssue {
            path: root.into(),
            line: 0,
            message,
        },
    };
    match observed {
        Err(error) => Ok(invalid(
            documents,
            format!("declared source refused: {error:?}"),
        )),
        Ok(value) if value.complete => Ok(AuthoredSourceObservation::Available {
            facts: value.facts,
            documents,
        }),
        Ok(value) => {
            let issue = value
                .documents
                .iter()
                .find(|(_, row)| {
                    !matches!(
                        row,
                        DeclaredDocumentObservation::Facts { .. }
                            | DeclaredDocumentObservation::Raw { .. }
                            | DeclaredDocumentObservation::OrdinaryMarkdown { .. }
                            | DeclaredDocumentObservation::Unclassified { .. }
                    )
                })
                .map(|(path, row)| {
                    format!(
                        "declared source {path} is incomplete: {}",
                        match row {
                            DeclaredDocumentObservation::NeedsInput { .. } =>
                                "ambiguous interpretation needs input",
                            DeclaredDocumentObservation::Unavailable { .. } => "source unavailable",
                            DeclaredDocumentObservation::Invalid { .. } =>
                                "selected syntax invalid",
                            _ => "unsupported reader",
                        }
                    )
                })
                .unwrap_or_else(|| "declared source census is incomplete".into());
            Ok(invalid(documents, issue))
        }
    }
}
