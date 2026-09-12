//! What the real parsers accept, asked of the real parsers.

use super::*;

#[test]
fn the_manifest_surface_holds_the_tables_a_manifest_declares() {
    let fields = manifest_fields();
    for expected in ["package", "project", "requires", "i18n", "compatibility"] {
        assert!(
            fields.iter().any(|f| f == expected),
            "`{expected}` is a table `vibe.toml` accepts and the probe did not find it; \
             found {} path(s)",
            fields.len()
        );
    }
}

#[test]
fn the_probe_opens_a_table_and_records_what_is_under_it() {
    let fields = manifest_fields();
    for expected in ["package.title", "package.name", "package.kind"] {
        assert!(
            fields.iter().any(|f| f == expected),
            "`{expected}` is a field of `[package]` and the probe did not reach it"
        );
    }
}

#[test]
fn an_array_of_tables_is_spelled_as_one() {
    let fields = manifest_fields();
    assert!(
        fields.iter().any(|f| f.starts_with("documents[]")),
        "`[[documents]]` is an array of tables and the surface must say so; \
         found: {:?}",
        fields
            .iter()
            .filter(|f| f.starts_with("documents"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_lock_file_surface_holds_its_own_tables() {
    let fields = lock_fields();
    for expected in ["meta", "meta.schema_version", "package[]", "package[].name"] {
        assert!(
            fields.iter().any(|f| f == expected),
            "`{expected}` is a path `vibe.lock` accepts and the probe did not find it; \
             found {} path(s)",
            fields.len()
        );
    }
}

/// A level whose members are not all optional still answers with its
/// whole list.
///
/// `vibe.lock` is the document that tests this, because `[meta]` is
/// required: were the refusal to arrive as «missing field `meta`»
/// instead of «unknown field … expected `meta` or `package`», the probe
/// would read it as «this is not a struct» and the whole lock-file
/// surface would collapse to nothing — silently, since an empty list and
/// a leaf print the same. Asserted on the refusal itself rather than on
/// the count, so an upgrade of serde or toml that reordered the two
/// checks turns this test red instead of turning the surface empty.
#[test]
fn a_required_member_does_not_hide_the_list() {
    let mut probes = 0usize;
    assert_eq!(
        accepted::<vibe_core::manifest::Lockfile>(&[], &mut probes),
        Some(vec!["meta".to_string(), "package".to_string()]),
        "the top level of `vibe.lock` has one required member and one optional one; \
         the refusal must still name both"
    );
}

/// An array of tables is recorded in ONE spelling — the one the document
/// actually accepts.
///
/// `vibe.lock` writes `[[package]]`, and a plain `[package]` is refused
/// («invalid type: map, expected a sequence»). Recording the bare name
/// beside the bracketed one would put a key in the surface that the
/// parser rejects, and `vibe doc diff` would then report a page owed to
/// a field that never existed.
#[test]
fn an_array_of_tables_is_recorded_bracketed_and_not_bare() {
    let fields = lock_fields();
    assert!(
        fields.iter().any(|f| f == "package[]"),
        "`[[package]]` is what `vibe.lock` accepts; found: {:?}",
        fields
            .iter()
            .filter(|f| f.starts_with("package"))
            .take(3)
            .collect::<Vec<_>>()
    );
    assert!(
        !fields.iter().any(|f| f == "package"),
        "`[package]` as a plain table is refused by `vibe.lock`, so the surface \
         must not claim it"
    );
}

#[test]
fn the_two_documents_do_not_answer_the_same() {
    assert_ne!(
        manifest_fields(),
        lock_fields(),
        "`vibe.toml` and `vibe.lock` are different contracts; one answer for both \
         means the probe asked neither"
    );
}

#[test]
fn a_reading_is_the_same_reading_twice() {
    assert_eq!(manifest_fields(), manifest_fields());
}

#[test]
fn a_refusal_that_is_not_about_a_key_is_not_a_field_list() {
    assert_eq!(expected("invalid type: map, expected a string"), None);
    assert_eq!(expected("missing field `name`"), None);
}

#[test]
fn a_struct_with_no_fields_is_still_a_struct() {
    let message = format!("unknown field `{PROBE}`, there are no fields");
    assert_eq!(expected(&message), Some(Vec::new()));
}

#[test]
fn the_field_list_is_read_out_of_the_refusal() {
    let message =
        format!("unknown field `{PROBE}`, expected one of `a`, `b-c`, `d` at line 1, column 1");
    assert_eq!(
        expected(&message),
        Some(vec!["a".to_string(), "b-c".to_string(), "d".to_string()])
    );
}
