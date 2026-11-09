use super::merge_preserving_comments;

#[test]
fn header_comments_survive_full_rewrite() {
    let existing = "\
# This is my project's vibe.toml.
# Edit with care.

[project]
name = \"old\"
version = \"0.0.1\"
";
    let new_rendered = "\
[project]
name = \"new\"
version = \"0.0.1\"
";
    let merged = merge_preserving_comments(existing, new_rendered);
    assert!(
        merged.contains("# This is my project's vibe.toml."),
        "header comment must survive:\n{merged}"
    );
    assert!(merged.contains("# Edit with care."));
    // The new value (`name = "new"`) wins over the old one.
    assert!(merged.contains("name = \"new\""));
    assert!(!merged.contains("name = \"old\""));
}

#[test]
fn pre_table_comments_survive_for_unchanged_sections() {
    let existing = "\
[project]
name = \"demo\"
version = \"0.0.1\"

# Primary registry — host migrated from GitVerse on 2026-04-29.
[[registry]]
name = \"vibespecs\"
url = \"https://github.com/vibespecs\"
";
    // Simulate `vibe install flow:wal` adding a [requires] section
    // — re-render the manifest with all sections, including the new
    // one.
    let new_rendered = "\
[project]
name = \"demo\"
version = \"0.0.1\"

[[registry]]
name = \"vibespecs\"
url = \"https://github.com/vibespecs\"

[requires]
packages = [\"flow:wal@^0.1.0\"]
";
    let merged = merge_preserving_comments(existing, new_rendered);
    assert!(
        merged.contains("# Primary registry — host migrated from GitVerse on 2026-04-29."),
        "pre-table comment on [[registry]] must survive:\n{merged}"
    );
    assert!(merged.contains("[requires]"));
    assert!(merged.contains("flow:wal@^0.1.0"));
}

#[test]
fn trailing_comments_survive() {
    let existing = "\
[project]
name = \"demo\"
version = \"0.0.1\"

# Footer — please don't remove this.
";
    let new_rendered = "\
[project]
name = \"demo\"
version = \"0.0.2\"
";
    let merged = merge_preserving_comments(existing, new_rendered);
    assert!(
        merged.contains("# Footer — please don't remove this."),
        "trailing comment must survive:\n{merged}"
    );
    assert!(merged.contains("version = \"0.0.2\""));
}

#[test]
fn inline_kv_comments_survive_inside_array_of_tables() {
    // The headline use case for inline-decor preservation:
    // operator hand-edited a comment between `name` and `url`
    // inside a `[[registry]]` block. A subsequent
    // `vibe install` re-renders the manifest; the inline
    // comment must not be wiped out.
    let existing = "\
[project]
name = \"demo\"
version = \"0.0.1\"

[[registry]]
name = \"vibespecs\"
# host migrated from GitVerse on 2026-04-29 — keep this in sync.
url = \"https://github.com/vibespecs\"
";
    // `vibe install`-shape rewrite: same registry, but with a
    // freshly-added `[requires]` block at the bottom.
    let new_rendered = "\
[project]
name = \"demo\"
version = \"0.0.1\"

[[registry]]
name = \"vibespecs\"
url = \"https://github.com/vibespecs\"

[requires]
packages = [\"flow:wal@^0.1.0\"]
";
    let merged = merge_preserving_comments(existing, new_rendered);
    assert!(
        merged.contains("# host migrated from GitVerse on 2026-04-29 — keep this in sync."),
        "inline comment between name and url must survive:\n{merged}"
    );
    assert!(merged.contains("[requires]"));
    assert!(merged.contains("flow:wal@^0.1.0"));
}

#[test]
fn nested_extension_tables_preserve_comments_and_new_values_recursively() {
    let existing = r#"[[extension]] # KEEP-EXTENSION-HEADER
id = "announce"
point = "phase:build"

# KEEP-HANDLER-PREFIX
[extension.handler] # KEEP-HANDLER-HEADER
kind = "builtin" # KEEP-HANDLER-KIND
# KEEP-HANDLER-NAME
name = "log"

# KEEP-CONFIG-PREFIX
[extension.config]
# KEEP-CONFIG-MESSAGE
message = "old message"

# KEEP-PASS-PREFIX
[extension.pass]
kind = "transform" # KEEP-PASS-KIND
level = "document"

# KEEP-PREBUILT-PREFIX
[extension.handler.prebuilt]
# KEEP-PREBUILT-PLATFORM
windows = "old.dll"
"#;
    let new_rendered = r#"[[extension]]
id = "announce"
point = "phase:build"

[extension.handler]
kind = "builtin"
name = "log"

[extension.config]
message = "new message"

[extension.pass]
kind = "transform"
level = "document"

[extension.handler.prebuilt]
windows = "new.dll"
"#;

    let merged = merge_preserving_comments(existing, new_rendered);
    for marker in [
        "KEEP-EXTENSION-HEADER",
        "KEEP-HANDLER-PREFIX",
        "KEEP-HANDLER-HEADER",
        "KEEP-HANDLER-KIND",
        "KEEP-HANDLER-NAME",
        "KEEP-CONFIG-PREFIX",
        "KEEP-CONFIG-MESSAGE",
        "KEEP-PASS-PREFIX",
        "KEEP-PASS-KIND",
        "KEEP-PREBUILT-PREFIX",
        "KEEP-PREBUILT-PLATFORM",
    ] {
        assert!(merged.contains(marker), "lost {marker}:\n{merged}");
    }
    assert!(merged.contains("message = \"new message\""), "{merged}");
    assert!(merged.contains("windows = \"new.dll\""), "{merged}");
    assert!(!merged.contains("old message"), "{merged}");
    assert!(!merged.contains("old.dll"), "{merged}");
}

#[test]
fn inline_table_preserves_inner_and_outer_decoration() {
    let existing = r#"[[extension]]
id = "inline"
handler = { kind=  "builtin" , name =  "log" } # KEEP-OUTER
"#;
    let new_rendered = r#"[[extension]]
id = "inline"
handler = { kind = "builtin", name = "log" }
"#;

    let merged = merge_preserving_comments(existing, new_rendered);
    assert!(
        merged.contains("handler = { kind=  \"builtin\" , name =  \"log\" } # KEEP-OUTER"),
        "inline-table decor must survive byte-for-byte:\n{merged}"
    );
}

#[test]
fn equal_length_nested_arrays_preserve_element_decoration_and_new_values() {
    let existing = r#"[[extension]]
id = "array"

[extension.config]
paths = [
    "old-alpha", # KEEP-ALPHA
    # KEEP-BETA
    "old-beta",
]
"#;
    let new_rendered = r#"[[extension]]
id = "array"

[extension.config]
paths = ["new-alpha", "new-beta"]
"#;

    let merged = merge_preserving_comments(existing, new_rendered);
    assert!(merged.contains("KEEP-ALPHA"), "{merged}");
    assert!(merged.contains("KEEP-BETA"), "{merged}");
    assert!(merged.contains("new-alpha"), "{merged}");
    assert!(merged.contains("new-beta"), "{merged}");
    assert!(!merged.contains("old-alpha"), "{merged}");
    assert!(!merged.contains("old-beta"), "{merged}");
}

/// Inline-to-header is the serialiser's own canonicalisation, not a type
/// change the operator made: the row keeps its comments. This assertion
/// used to run the other way — it pinned the loss — which meant the first
/// rewrite after someone authored `config = { … }` the way the docs show
/// deleted their comment without saying so.
#[test]
fn canonicalised_inline_rows_keep_the_comments_of_the_line_they_replace() {
    let existing = r#"[[extension]]
id = "shape"
# KEEP-CANONICALISED-KEY
config = { mode = "old" } # KEEP-CANONICALISED-TRAILING
"#;
    let new_rendered = r#"[[extension]]
id = "shape"

[extension.config]
mode = "new"
"#;
    let merged = merge_preserving_comments(existing, new_rendered);
    assert!(merged.contains("KEEP-CANONICALISED-KEY"), "{merged}");
    assert!(merged.contains("KEEP-CANONICALISED-TRAILING"), "{merged}");
    assert!(merged.contains("mode = \"new\""), "{merged}");
    assert!(!merged.contains("mode = \"old\""), "{merged}");
    // Still a valid document after the comments moved onto the header.
    assert!(merged.parse::<toml_edit::DocumentMut>().is_ok(), "{merged}");
}

/// A canonicalised array carries its own trailing note to the last header
/// it expands into — and a *true* mismatch beside it still refuses to
/// cross, so the bridge cannot become a general decoration leak.
#[test]
fn array_trailing_notes_move_but_mismatches_still_refuse() {
    let existing = r#"[[extension]]
id = "shape"
inputs = [{ path = "a" }, { path = "b" }] # KEEP-ARRAY-NOTE
legacy = "old" # DROP-RENAMED-KEY
"#;
    let new_rendered = r#"[[extension]]
id = "shape"
replacement = "new"

[[extension.inputs]]
path = "a"

[[extension.inputs]]
path = "b"
"#;
    let merged = merge_preserving_comments(existing, new_rendered);
    assert_eq!(merged.matches("KEEP-ARRAY-NOTE").count(), 1, "{merged}");
    assert!(!merged.contains("DROP-RENAMED-KEY"), "{merged}");
    assert!(merged.contains("replacement = \"new\""), "{merged}");
    assert!(merged.parse::<toml_edit::DocumentMut>().is_ok(), "{merged}");
}

#[test]
fn decoration_does_not_cross_key_or_array_length_mismatches() {
    let key_existing = r#"[[extension]]
id = "key"
legacy = "old" # DROP-KEY-MISMATCH
"#;
    let key_new = r#"[[extension]]
id = "key"
replacement = "new"
"#;
    let key_merged = merge_preserving_comments(key_existing, key_new);
    assert!(!key_merged.contains("DROP-KEY-MISMATCH"));
    assert!(key_merged.contains("replacement = \"new\""));

    let length_existing = r#"[[extension]]
id = "length"

[extension.config]
# DROP-LENGTH-KEY
paths = [
    "one", # DROP-LENGTH-MISMATCH
    "two",
]
"#;
    let length_new = r#"[[extension]]
id = "length"

[extension.config]
paths = ["one", "two", "three"]
"#;
    let length_merged = merge_preserving_comments(length_existing, length_new);
    assert!(!length_merged.contains("DROP-LENGTH-KEY"));
    assert!(!length_merged.contains("DROP-LENGTH-MISMATCH"));
    assert!(length_merged.contains("three"));
}

#[test]
fn merge_falls_back_safely_on_invalid_existing() {
    // If the existing file is unparseable garbage, the merge
    // returns the new rendering unchanged.
    let existing = "this is not valid TOML !@#";
    let new_rendered = "[project]\nname = \"x\"\nversion = \"0.1.0\"\n";
    let merged = merge_preserving_comments(existing, new_rendered);
    assert_eq!(merged, new_rendered);
}

#[test]
fn merge_falls_back_safely_on_invalid_new() {
    // Same direction — defensive against a bug in the serde
    // serialiser producing something toml_edit can't parse.
    let existing = "[project]\nname = \"x\"\n";
    let new_rendered = "this is not valid TOML !@#";
    let merged = merge_preserving_comments(existing, new_rendered);
    assert_eq!(merged, new_rendered);
}
