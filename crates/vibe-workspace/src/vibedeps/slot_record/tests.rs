use tempfile::tempdir;

use super::*;

fn hash(digit: char) -> String {
    std::iter::repeat_n(digit, 64).collect()
}

fn mixed(files: Vec<SlotFile>) -> SlotRecord {
    SlotRecord {
        schema: SLOT_RECORD_SCHEMA,
        source_hash: content_hash('1'),
        spec_format: SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files,
    }
}

fn row(path: &str) -> SlotFile {
    SlotFile {
        path: path.to_string(),
        sha256: hash('a'),
        source: None,
        disposition: None,
    }
}

fn content_hash(digit: char) -> ContentHash {
    ContentHash::parse(&format!("sha256:{}", hash(digit))).unwrap()
}

fn assert_invalid_on_read_and_write(record: SlotRecord, expected: &str) {
    let slot = tempdir().unwrap();
    let write_error = write_slot_record(slot.path(), &record).unwrap_err();
    assert!(write_error.contains(expected), "{write_error}");

    let wire = toml::to_string_pretty(&record_to_wire(&record)).unwrap();
    fs::write(slot.path().join(SLOT_RECORD_FILENAME), wire).unwrap();
    let read_error = read_slot_record(slot.path()).unwrap_err();
    assert!(read_error.contains(expected), "{read_error}");
}

#[test]
fn mixed_round_trip_uses_exact_file_table_spelling() {
    let slot = tempdir().unwrap();
    let record = mixed(vec![
        row("README.md"),
        row("vibevm/vibespecs/boot/20-stack.xml"),
    ]);
    write_slot_record(slot.path(), &record).unwrap();

    let wire = fs::read_to_string(slot.path().join(SLOT_RECORD_FILENAME)).unwrap();
    assert_eq!(wire.matches("[[file]]").count(), 2);
    assert!(!wire.contains("[[files]]"));
    assert!(!wire.contains("converter_recipe"));
    assert!(!wire.contains("source ="));
    assert_eq!(read_slot_record(slot.path()).unwrap(), record);
}

#[test]
fn transformed_round_trip_records_copied_and_converted_rows() {
    let slot = tempdir().unwrap();
    let record = SlotRecord {
        schema: SLOT_RECORD_SCHEMA,
        source_hash: content_hash('1'),
        spec_format: SpecFormat::Xml,
        converter_recipe: Some("specdoc/4".to_string()),
        derived_hash: Some(content_hash('2')),
        overlay_hash: Some(content_hash('3')),
        files: vec![
            SlotFile {
                path: "README.md".to_string(),
                sha256: hash('b'),
                source: Some("README.md".to_string()),
                disposition: Some(SlotFileDisposition::Copied),
            },
            SlotFile {
                path: "vibevm/vibespecs/boot/20-stack.xml".to_string(),
                sha256: hash('c'),
                source: Some("vibevm/vibespecs/boot/20-stack.md".to_string()),
                disposition: Some(SlotFileDisposition::Converted),
            },
        ],
    };
    write_slot_record(slot.path(), &record).unwrap();

    let wire = fs::read_to_string(slot.path().join(SLOT_RECORD_FILENAME)).unwrap();
    assert!(wire.contains("disposition = \"copied\""));
    assert!(wire.contains("disposition = \"converted\""));
    assert_eq!(read_slot_record(slot.path()).unwrap(), record);
}

#[test]
fn unknown_top_level_field_is_refused() {
    let slot = tempdir().unwrap();
    fs::write(
        slot.path().join(SLOT_RECORD_FILENAME),
        "schema = 1\nsource_hash = \"sha256:1111111111111111111111111111111111111111111111111111111111111111\"\nspec_format = \"mixed\"\nunknown = true\n",
    )
    .unwrap();
    let error = read_slot_record(slot.path()).unwrap_err();
    assert!(error.contains("slot record parse failure"), "{error}");
    assert!(error.contains("unknown field"), "{error}");
}

#[test]
fn unknown_file_field_is_refused() {
    let slot = tempdir().unwrap();
    fs::write(
        slot.path().join(SLOT_RECORD_FILENAME),
        format!(
            "schema = 1\nsource_hash = \"sha256:1111111111111111111111111111111111111111111111111111111111111111\"\nspec_format = \"mixed\"\n\
             [[file]]\npath = \"README.md\"\nsha256 = \"{}\"\nunknown = true\n",
            hash('d')
        ),
    )
    .unwrap();
    let error = read_slot_record(slot.path()).unwrap_err();
    assert!(error.contains("slot record parse failure"), "{error}");
    assert!(error.contains("unknown field"), "{error}");
}

#[test]
fn wrong_schema_is_refused_on_read_and_write() {
    let mut record = mixed(vec![]);
    record.schema = 2;
    assert_invalid_on_read_and_write(record, "schema 2 is unsupported; expected 1");
}

#[test]
fn malformed_content_hash_and_invalid_file_hash_are_refused() {
    let slot = tempdir().unwrap();
    fs::write(
        slot.path().join(SLOT_RECORD_FILENAME),
        "schema = 1\nsource_hash = \"sha256:not-hex\"\nspec_format = \"mixed\"\nfile = []\n",
    )
    .unwrap();
    let source_error = read_slot_record(slot.path()).unwrap_err();
    assert!(
        source_error.contains("slot record invariant failure")
            && source_error.contains("source_hash is invalid"),
        "{source_error}"
    );

    let mut invalid_file_hash = mixed(vec![row("README.md")]);
    invalid_file_hash.files[0].sha256 = hash('A');
    assert_invalid_on_read_and_write(invalid_file_hash, "exactly 64 lowercase hexadecimal digits");
}

#[test]
fn unsorted_duplicate_and_unsafe_paths_are_refused() {
    let cases = [
        (mixed(vec![row("b"), row("a")]), "not strictly sorted"),
        (mixed(vec![row("a"), row("a")]), "duplicates"),
        (mixed(vec![row("../escape")]), "`.` or `..` component"),
        (mixed(vec![row("a\\b")]), "must use forward slashes"),
        (mixed(vec![row("/absolute")]), "must be relative"),
        (mixed(vec![row("C:/absolute")]), "must be relative"),
        (
            mixed(vec![row(SLOT_RECORD_FILENAME)]),
            "is reserved for the slot record",
        ),
        (mixed(vec![row("nul\0path")]), "contains a NUL byte"),
    ];
    for (record, expected) in cases {
        assert_invalid_on_read_and_write(record, expected);
    }
}

#[test]
fn mixed_and_transformed_field_shape_violations_are_refused() {
    let mut mixed_metadata = mixed(vec![]);
    mixed_metadata.converter_recipe = Some("specdoc/4".to_string());

    let mut mixed_row = mixed(vec![row("README.md")]);
    mixed_row.files[0].source = Some("README.md".to_string());

    let transformed = |converter_recipe: Option<&str>, derived_hash: Option<&str>| SlotRecord {
        schema: SLOT_RECORD_SCHEMA,
        source_hash: content_hash('1'),
        spec_format: SpecFormat::Markdown,
        converter_recipe: converter_recipe.map(str::to_string),
        derived_hash: derived_hash.map(|_| content_hash('2')),
        overlay_hash: None,
        files: vec![SlotFile {
            path: "README.md".to_string(),
            sha256: hash('e'),
            source: Some("README.xml".to_string()),
            disposition: Some(SlotFileDisposition::Converted),
        }],
    };
    let mut missing_row_field = transformed(Some("specdoc/4"), Some("present"));
    missing_row_field.files[0].disposition = None;

    let cases = [
        (mixed_metadata, "mixed records must omit"),
        (mixed_row, "mixed file[0] must omit"),
        (
            transformed(None, Some("present")),
            "require converter_recipe",
        ),
        (transformed(Some("specdoc/4"), None), "require derived_hash"),
        (missing_row_field, "requires both source and disposition"),
    ];
    for (record, expected) in cases {
        assert_invalid_on_read_and_write(record, expected);
    }

    let mut unsafe_source = transformed(Some("specdoc/4"), Some("present"));
    unsafe_source.files[0].source = Some("../README.xml".to_string());
    assert_invalid_on_read_and_write(unsafe_source, "`.` or `..` component");
}

#[test]
fn sha256_file_hashes_abc() {
    let slot = tempdir().unwrap();
    let path = slot.path().join("abc.txt");
    fs::write(&path, b"abc").unwrap();
    assert_eq!(
        sha256_file(&path).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
