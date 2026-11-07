//! Facts from the existing installed binding; witnesses name observed output bytes.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-QUERY-READ",
    r = 1
);
use super::*;
use vibe_core::manifest::SourceInterpretation;
use vibe_spec::{FileResolver, ResolveError};

/// Observe the selected installed inventory. Logical source paths determine
/// addresses and witness labels; witness hashes describe materialized bytes,
/// never an inferred original-file digest. Record/read failures remain errors;
/// invalid native syntax retains the complete captured document witnesses.
pub fn observe_installed_source(
    resolver: &FileResolver,
    group: &str,
    name: &str,
) -> Result<AuthoredSourceObservation, ResolveError> {
    let binding = resolver.installed_source_binding(group, name)?;
    let expected = binding.expectation();
    let mut rows = binding
        .outputs()
        .iter()
        .filter(|row| row.interpretation.is_some())
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| a.source_path.cmp(&b.source_path));
    let mut captured = Vec::new();
    for row in rows {
        let bytes = resolver.read_installed_source(
            expected.group.as_str(),
            &expected.name,
            &row.source_path,
            None,
        )?;
        captured.push((row, bytes));
    }
    let documents = captured
        .iter()
        .map(|(row, bytes)| SourceFileWitness::of(&row.source_path, bytes))
        .collect::<Vec<_>>();
    let coordinate = format!("{}/{}", expected.group, expected.name);
    let mut facts = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut issue = None;
    for (row, bytes) in &captured {
        let Some(interpretation @ (SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)) =
            row.interpretation
        else {
            continue;
        };
        let vocabulary = if interpretation == SourceInterpretation::VibeDoc {
            vibe_specdoc::doc::Vocabulary::Doc
        } else {
            vibe_specdoc::doc::Vocabulary::Spec
        };
        let parsed = std::str::from_utf8(bytes)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                super::declared::parse_native_document(text, &row.media, vocabulary)
                    .map_err(|error| error.to_string())
            });
        let parsed = parsed.and_then(|doc| {
            address_prefix(&coordinate, &row.source_path)
                .map(|prefix| authored_facts(&doc, &prefix))
                .map_err(|error| error.to_string())
        });
        match parsed {
            Ok(rows) => {
                for fact in rows {
                    if !seen.insert(fact.address.clone()) {
                        issue = Some(SourceIssue {
                            path: row.source_path.clone().into(),
                            line: 0,
                            message: format!("duplicate full fact address `{}`", fact.address),
                        });
                        break;
                    }
                    facts.push(fact);
                }
            }
            Err(message) => {
                issue = Some(SourceIssue {
                    path: row.source_path.clone().into(),
                    line: 0,
                    message,
                })
            }
        }
        if issue.is_some() {
            break;
        }
    }
    resolver.recheck_installed_package(expected.group.as_str(), &expected.name)?;
    Ok(match issue {
        Some(issue) => AuthoredSourceObservation::Invalid { documents, issue },
        None => {
            facts.sort_by(|a, b| a.address.cmp(&b.address));
            AuthoredSourceObservation::Available { facts, documents }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_bound_source_refuses_instead_of_falling_back_to_suffix_scan() {
        let root = tempfile::tempdir().unwrap();
        let resolver = FileResolver::new(
            root.path(),
            vibe_spec::SelfCoordinate::new(Some("org.host".into()), "host".into()),
        );
        assert!(observe_installed_source(&resolver, "org.example", "package").is_err());
    }
}
