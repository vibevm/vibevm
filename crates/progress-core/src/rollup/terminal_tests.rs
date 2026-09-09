use super::*;
use crate::parse::parse_document;

#[test]
fn unanimous_document_fold_cannot_erase_requirements() {
    let doc = parse_document(
        "spec/t.md",
        "<status stage=\"spec\" state=\"done\"/>\n\n# T {#t}\n\n@fact:A body @requires:specification @spec/done\n",
    );
    let issues = fold_check(&doc);
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0].section, "spec/t.md");
    assert_eq!(issues[0].unit, "A");
    assert_eq!(issues[0].lost, FoldLoss::Requirements);
}

#[test]
fn mixed_document_representatives_are_not_a_fold() {
    let doc = parse_document(
        "spec/t.md",
        "<status stage=\"spec\" state=\"done\"/>\n\n# T {#t}\n\n@fact:A body @requires:specification @spec/done\n\n@fact:B body @impl/done\n",
    );
    assert!(fold_check(&doc).is_empty());
}

#[test]
fn marked_subsection_owns_its_classified_fold_without_document_duplicate() {
    let doc = parse_document(
        "spec/t.md",
        "# T {#t}\n\n<status stage=\"spec\" state=\"done\"/>\n\n## S {#s}\n\n<status stage=\"spec\" state=\"done\"/>\n\n@fact:A body @requires:specification @spec/done\n",
    );
    let issues = fold_check(&doc);
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0].section, "s");
    assert_eq!(issues[0].lost, FoldLoss::Requirements);
}

#[test]
fn unmarked_subsection_is_transparent_to_document_fold() {
    let doc = parse_document(
        "spec/t.md",
        "# T {#t}\n\n<status stage=\"spec\" state=\"done\"/>\n\n## S {#s}\n\n@fact:A body @requires:specification @spec/done\n",
    );
    let issues = fold_check(&doc);
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0].section, "spec/t.md");
    assert_eq!(issues[0].unit, "A");
}

#[test]
fn same_line_cells_name_the_exact_losing_fact() {
    let doc = parse_document(
        "spec/t.md",
        "# T {#t}\n\n## S {#s}\n\n<status stage=\"spec\" state=\"done\"/>\n\n| A | B |\n|---|---|\n| @fact:A old @spec/done | @fact:B classified @requires:specification @spec/done |\n",
    );
    let issues = fold_check(&doc);
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0].unit, "B");
    assert_eq!(issues[0].lost, FoldLoss::Requirements);
}
