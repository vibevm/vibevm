//! Loaded-source precedence is distinct from the row's original source index.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#precedence");
use super::*;

fn row(source: &str, index: usize, target: &str) -> RemapRow {
    let declaration: RowDecl = toml::from_str(&format!(
        "prefix='https://source-order.invalid/'\nurl='{target}'\n"
    ))
    .unwrap();
    declaration
        .into_row(Provenance::new(Layer::UserPackage, source, index))
        .unwrap()
}
fn source(name: &str, rows: Vec<RemapRow>) -> LoadedSource {
    LoadedSource {
        layer: Layer::UserPackage,
        source: name.into(),
        rows,
        policy: UnmatchedPolicy::Allow,
    }
}
fn artifact() -> Artifact {
    Artifact::new(ArtifactKind::AppInput).with_url("https://source-order.invalid/file")
}
fn assert_winner(locator: &Locator, expected: &RemapRow, position: usize, target: &str) {
    let input = artifact();
    let explanation = locator.explain(&input);
    assert_eq!(explanation.chosen.as_ref().unwrap().display(), target);
    assert!(!explanation.denied);
    let winner = &explanation.matches[0];
    assert_eq!(winner.row.provenance, expected.from);
    assert_eq!(winner.row.position, position);
    assert_eq!(locator.row(&winner.row), Some(expected));
    let Resolution::Candidates(candidates) = locator.locate(&input) else {
        panic!("the fixture contains no deny rows");
    };
    assert_eq!(candidates[0].target.display(), target);
    assert_eq!(candidates[0].row.as_ref(), Some(&winner.row));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn later_same_layer_source_wins_when_both_local_rows_are_zero() {
    let first = row("low-priority-package", 0, "file:///first/");
    let later = row("high-priority-package", 0, "file:///later/");
    let locator = Locator::from_sources(
        vec![
            source("low-priority-package", vec![first.clone()]),
            source("high-priority-package", vec![later.clone()]),
        ],
        Vec::new(),
    );
    assert_winner(&locator, &later, 1, "file:///later/file");
    assert_eq!(later.from.index, 0);
    assert_eq!(locator.rows(), &[first, later]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn later_source_outranks_an_earlier_sources_larger_local_index() {
    let first = row("earlier-source", 99, "file:///earlier/");
    let later = row("later-source", 0, "file:///later/");
    let locator = Locator::from_sources(
        vec![
            source("earlier-source", vec![first.clone()]),
            source("later-source", vec![later.clone()]),
        ],
        Vec::new(),
    );
    assert_winner(&locator, &later, 1, "file:///later/file");
    let explanation = locator.explain(&artifact());
    assert_eq!(explanation.matches[0].row.provenance.index, 0);
    assert_eq!(explanation.matches[1].row.provenance.index, 99);
    assert_eq!(explanation.matches[1].row.position, 0);
    assert_eq!(locator.rows(), &[first, later]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn same_source_reversed_vector_preserves_the_higher_local_index_winner() {
    let high_index = row("one-source", 9, "file:///declared-later/");
    let low_index = row("one-source", 2, "file:///declared-earlier/");
    let locator = Locator::from_sources(
        vec![source(
            "one-source",
            vec![high_index.clone(), low_index.clone()],
        )],
        Vec::new(),
    );
    assert_winner(&locator, &high_index, 0, "file:///declared-later/file");
    let explanation = locator.explain(&artifact());
    assert_eq!(explanation.matches[0].row.provenance.index, 9);
    assert_eq!(explanation.matches[1].row.provenance.index, 2);
    assert_eq!(locator.rows(), &[high_index, low_index]);
}
