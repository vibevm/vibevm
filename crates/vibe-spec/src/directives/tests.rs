use super::*;

#[test]
fn parses_the_three_directives() {
    let src = "\
#embed spec://vibevm/a/b#x
#use spec://org.vibevm.demo/lib/contract/API#root
#source spec://vibevm/c/d#y
";
    let d = Directives::parse(src);
    assert_eq!(d.directives.len(), 3);
    assert_eq!(d.errors, vec![]);
    assert_eq!(d.directives[0].kind, DirectiveKind::Embed);
    assert_eq!(d.directives[1].kind, DirectiveKind::Use);
    assert_eq!(d.directives[2].kind, DirectiveKind::Source);
    assert_eq!(d.directives[0].address.doc_path, "a/b");
    assert_eq!(d.directives[1].line, 1);
}

#[test]
fn parses_options() {
    let d = Directives::parse("#embed once spec://vibevm/a/b#x\n");
    assert_eq!(d.directives[0].options, "once");
    assert_eq!(d.directives[0].address.doc_path, "a/b");
}

#[test]
fn collects_in_place_uses_from_prose() {
    let src = "See @spec://vibevm/common/PROP-000#commits for the rules.\n";
    let d = Directives::parse(src);
    assert_eq!(d.in_place_uses.len(), 1);
    assert_eq!(d.in_place_uses[0].address.doc_path, "common/PROP-000");
    assert_eq!(d.in_place_uses[0].address.anchor, vec!["commits"]);
}

#[test]
fn trims_brackets_and_sentence_punctuation() {
    let d = Directives::parse("(@spec://vibevm/a/b#c).\n");
    assert_eq!(d.in_place_uses.len(), 1);
    assert_eq!(
        d.in_place_uses[0].address.without_pin(),
        "spec://vibevm/a/b#c"
    );
}

/// R2 probe (B-011): what does the existing `@spec://` scan do inside inline
/// backticks? The closing backtick terminates `address_run`, so the run
/// parses — the use IS collected. The `@!` scanner must mirror this. This
/// test pins the discovered behaviour so the mirror stays honest.
#[test]
fn probe_spec_in_inline_backticks_is_collected() {
    let d = Directives::parse("see `@spec://vibevm/a/b#c` here\n");
    assert_eq!(d.in_place_uses.len(), 1);
    assert_eq!(d.in_place_uses[0].address.doc_path, "a/b");
    assert_eq!(d.in_place_uses[0].address.anchor, vec!["c"]);
}

#[test]
fn multiple_in_place_uses_on_one_line() {
    let d = Directives::parse("@spec://vibevm/a#x and @spec://vibevm/b#y\n");
    assert_eq!(d.in_place_uses.len(), 2);
}

#[test]
fn bare_spec_is_not_an_in_place_use() {
    // No `@` sigil → discretionary reference, not collected.
    let d = Directives::parse("see spec://vibevm/a/b#c here\n");
    assert!(d.in_place_uses.is_empty());
    assert!(d.directives.is_empty());
}

#[test]
fn directives_in_fences_are_ignored() {
    let src = "\
#use spec://vibevm/real#x
```
#use spec://vibevm/fake#y
@spec://vibevm/fake#z
```
";
    let d = Directives::parse(src);
    assert_eq!(d.directives.len(), 1);
    assert_eq!(d.directives[0].address.doc_path, "real");
    assert!(d.in_place_uses.is_empty());
}

#[test]
fn heading_is_not_a_directive() {
    // A real heading (`# text`, space after `#`) is not a directive.
    let d = Directives::parse("# Use the thing {#use-it}\nbody\n");
    assert!(d.directives.is_empty());
}

#[test]
fn html_comments_mask_directives_and_sigils() {
    // The compiled lane's resolution preamble quotes `#use … as X` and
    // `@!X` verbatim inside a multi-line HTML comment; provenance markers
    // carry addresses in single-line comments. None of it is authored
    // directive text — the scanner must collect nothing and error on
    // nothing (the exact false positive that broke the first host
    // regeneration on the nested git-practices lane).
    let src = "\
<!-- RESOLUTION RULES — read these five lines before anything else:
  4. `#use spec://vibevm/a/b#c as X` binds a file-local alias; `@!X` is a
 mandatory read of X's target (same rules as @spec://vibevm/d/e#f).
-->
<!-- vibe:static org.example/pkg — vibedeps/pkg/1.0.0/spec/boot/x.md -->
real body with @spec://vibevm/real#one
#use spec://vibevm/real#two
";
    let d = Directives::parse(src);
    assert_eq!(d.errors, vec![], "{:?}", d.errors);
    assert!(d.aliases.is_empty(), "a quoted `as X` is not a declaration");
    assert_eq!(d.directives.len(), 1, "only the real #use counts");
    assert_eq!(d.directives[0].address.doc_path, "real");
    assert_eq!(d.in_place_uses.len(), 1, "only the real @spec counts");
    assert_eq!(d.in_place_uses[0].address.anchor, vec!["one"]);
}

#[test]
fn a_mid_line_comment_does_not_mask_the_line() {
    // Line-grained on purpose: a content line that merely CONTAINS a
    // closed `<!-- -->` is a content line; its in-place uses still count.
    let d = Directives::parse("see <!-- note --> @spec://vibevm/a#x here\n");
    assert_eq!(d.in_place_uses.len(), 1);
}

#[test]
fn bad_address_is_reported() {
    // A digit-headed anchor: `#Bad` is a legal fact id and no longer serves.
    let d = Directives::parse("#use spec://vibevm/a/b#9lives\n");
    assert!(d.directives.is_empty());
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].message.contains("bad address"));
}

#[test]
fn directive_without_address_is_reported() {
    let d = Directives::parse("#embed nothing-here\n");
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].message.contains("no spec:// address"));
}

// ---- B-011: the `as` clause and the `@!X` sigil ------------------------

#[test]
fn as_clause_binds_alias() {
    let d = Directives::parse("#use spec://vibevm/a/b#c as root\n");
    assert_eq!(d.directives.len(), 1);
    assert_eq!(d.directives[0].kind, DirectiveKind::Use);
    assert_eq!(d.directives[0].options, "");
    assert_eq!(d.directives[0].address.doc_path, "a/b");
    assert_eq!(d.errors, vec![]);
    let addr = d.aliases.get("root").expect("alias `root` declared");
    assert_eq!(addr.without_pin(), "spec://vibevm/a/b#c");
}

#[test]
fn options_before_address_still_parse_with_as() {
    let d = Directives::parse("#use once spec://vibevm/a/b#c as root\n");
    assert_eq!(d.directives[0].options, "once");
    assert_eq!(d.directives[0].address.doc_path, "a/b");
    assert_eq!(
        d.aliases.get("root").map(|a| a.doc_path.clone()),
        Some("a/b".to_string())
    );
}

#[test]
fn duplicate_alias_is_reported_and_first_wins() {
    let src = "\
#use spec://vibevm/a/b#c as root
#use spec://vibevm/x/y#z as root
";
    let d = Directives::parse(src);
    // Both directives still land — each is a valid dependency edge.
    assert_eq!(d.directives.len(), 2);
    // ...but the alias table keeps the first declaration.
    assert_eq!(
        d.aliases.get("root").map(|a| a.doc_path.clone()),
        Some("a/b".to_string())
    );
    // Exactly one error naming the alias and both lines (1-based).
    assert_eq!(d.errors.len(), 1);
    let msg = &d.errors[0].message;
    assert!(msg.contains("duplicate alias"), "{msg}");
    assert!(msg.contains("`root`"), "{msg}");
    assert!(msg.contains("line 1"), "{msg}");
    assert!(msg.contains("line 2"), "{msg}");
}

#[test]
fn trailing_tokens_after_address_error_for_every_kind() {
    // R1: the silently-ignored tail is now an error, for each directive kind.
    for kw in ["#use", "#embed", "#source"] {
        let src = format!("{kw} spec://vibevm/a/b#c junk after\n");
        let d = Directives::parse(&src);
        assert!(
            d.directives.is_empty(),
            "{kw}: directive must not land on trailing junk"
        );
        assert_eq!(d.errors.len(), 1, "{kw}: expected one error");
        assert!(
            d.errors[0]
                .message
                .contains("unexpected tokens after address"),
            "{kw}: {}",
            d.errors[0].message
        );
    }
}

#[test]
fn as_clause_on_embed_and_source_errors() {
    // `as` is a `#use` clause; on `#embed`/`#source` it is a defect.
    for kw in ["#embed", "#source"] {
        let src = format!("{kw} spec://vibevm/a/b#c as root\n");
        let d = Directives::parse(&src);
        assert!(d.directives.is_empty(), "{kw} … as: must not land");
        assert_eq!(d.errors.len(), 1, "{kw} … as: expected one error");
        assert!(
            d.errors[0].message.contains("`as` is a `#use` clause"),
            "{kw}: {}",
            d.errors[0].message
        );
    }
}

#[test]
fn as_clause_without_name_or_with_bad_name_errors() {
    // `as` with no name.
    let d = Directives::parse("#use spec://vibevm/a/b#c as\n");
    assert!(d.directives.is_empty());
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].message.contains("needs an alias name"));

    // `as` with a digit-headed name (violates the identifier grammar).
    let d = Directives::parse("#use spec://vibevm/a/b#c as 9bad\n");
    assert!(d.directives.is_empty());
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].message.contains("not a valid identifier"));

    // `as` with more than one name.
    let d = Directives::parse("#use spec://vibevm/a/b#c as a b\n");
    assert!(d.directives.is_empty());
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].message.contains("exactly one alias name"));
}

#[test]
fn at_bang_resolves_alias_declared_later_in_file() {
    // Whole-file scope (B-011 §4): the `@!root` use appears BEFORE the
    // `as root` declaration — pass 2 resolves against the completed table.
    let src = "\
Sees @!root here on line 1.
#use spec://vibevm/a/b#c as root
";
    let d = Directives::parse(src);
    assert_eq!(d.errors, vec![]);
    assert_eq!(d.in_place_uses.len(), 1);
    assert_eq!(d.in_place_uses[0].address.doc_path, "a/b");
    assert_eq!(d.in_place_uses[0].address.anchor, vec!["c"]);
    assert_eq!(d.in_place_uses[0].line, 0);
}

#[test]
fn at_bang_undeclared_alias_lists_known_aliases() {
    let src = "\
#use spec://vibevm/a/b#c as root
Refers to @!missing and @!root.
";
    let d = Directives::parse(src);
    // `@!root` resolves; `@!missing` is the one error.
    assert_eq!(d.in_place_uses.len(), 1);
    assert_eq!(d.errors.len(), 1);
    let msg = &d.errors[0].message;
    assert!(msg.contains("undeclared alias"), "{msg}");
    assert!(msg.contains("@!missing"), "{msg}");
    assert!(msg.contains("root"), "{msg}"); // known alias is listed
}

#[test]
fn at_bang_undeclared_with_no_aliases_says_none_declared() {
    let d = Directives::parse("Refers to @!ghost.\n");
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].message.contains("undeclared alias"));
    assert!(d.errors[0].message.contains("none declared"));
}

#[test]
fn at_bang_in_fences_is_ignored() {
    let src = "\
#use spec://vibevm/a/b#c as root
```
@!root inside a fence
@spec://vibevm/fake#z
```
";
    let d = Directives::parse(src);
    assert_eq!(d.in_place_uses.len(), 0);
    assert_eq!(d.errors, vec![]);
}

#[test]
fn at_bang_in_inline_backticks_mirrors_spec() {
    // R2: `@!` mirrors `@spec://` — inline backticks do not suppress
    // collection (only fenced blocks do, via the fence mask). The closing
    // backtick terminates the run, so both sigils are collected.
    let src = "\
#use spec://vibevm/a/b#c as root
see `@!root` in code, like `@spec://vibevm/a/b#c` is.
";
    let d = Directives::parse(src);
    assert_eq!(d.errors, vec![]);
    assert_eq!(d.in_place_uses.len(), 2);
    assert_eq!(d.in_place_uses[0].address.doc_path, "a/b");
    assert_eq!(d.in_place_uses[1].address.doc_path, "a/b");
}

#[test]
fn at_bang_adjacent_punctuation_trims_to_identifier() {
    // R3: the identifier grammar naturally terminates the name; trailing
    // punctuation is prose, consistent with `address_run`'s trimming.
    let src = "\
#use spec://vibevm/a/b#c as root
Trailing dot @!root. and parenthesised (@!root) both resolve.
";
    let d = Directives::parse(src);
    assert_eq!(d.errors, vec![]);
    assert_eq!(d.in_place_uses.len(), 2);
    for u in &d.in_place_uses {
        assert_eq!(u.address.doc_path, "a/b");
        assert_eq!(u.address.anchor, vec!["c"]);
    }
}

#[test]
fn multiple_at_bang_uses_on_one_line() {
    let src = "\
#use spec://vibevm/a#x as a
#use spec://vibevm/b#y as b
@a and @!a and @!b here
";
    let d = Directives::parse(src);
    // `@a` is neither `@spec://` nor `@!` — not collected. Two `@!` uses.
    assert_eq!(d.errors, vec![]);
    assert_eq!(d.in_place_uses.len(), 2);
}

// ---- R5 (B-011): the compiled lane is not a citation target -----------

#[test]
fn use_into_the_compiled_lane_is_rejected() {
    // A directive whose address names a generated STATIC lane is rejected
    // (PROP-035 §11 ##COMPILED-LANE-IS-NOT-A-CITATION-TARGET).
    let d = Directives::parse("#use spec://vibevm/boot/STATIC#root\n");
    assert!(d.directives.is_empty(), "the directive must not land");
    assert_eq!(d.errors.len(), 1);
    let msg = &d.errors[0].message;
    assert!(msg.contains("not a citation target"), "{msg}");
    assert!(msg.contains("PROP-035 §11"), "{msg}");
    assert!(
        msg.contains("##COMPILED-LANE-IS-NOT-A-CITATION-TARGET"),
        "{msg}"
    );
}

#[test]
fn every_directive_kind_rejects_a_lane_target() {
    for kw in ["#use", "#embed", "#source"] {
        let d = Directives::parse(&format!("{kw} spec://org.example/pkg/boot/STATIC#root\n"));
        assert!(d.directives.is_empty(), "{kw}: must be rejected");
        assert_eq!(d.errors.len(), 1, "{kw}: one error");
        assert!(
            d.errors[0].message.contains("not a citation target"),
            "{kw}: {}",
            d.errors[0].message
        );
    }
}

#[test]
fn at_spec_into_the_compiled_lane_is_rejected() {
    // The in-place-use sigil shares the chokepoint with directives.
    let d = Directives::parse("See @spec://vibevm/boot/STATIC#root here.\n");
    assert!(d.in_place_uses.is_empty(), "the use must not land");
    assert_eq!(d.errors.len(), 1);
    assert!(d.errors[0].message.contains("not a citation target"));
}

#[test]
fn an_unrelated_path_ending_in_static_is_not_flagged() {
    // `STATIC` is a common word; only the `boot/STATIC` lane path is illegal,
    // and only at a path boundary — `foo/boot/STATIC` matches, `boot/STATIC`
    // matches, but `notboot/STATIC` does not (no boundary) and a doc named
    // `STATIC` at a different path does not either.
    let d = Directives::parse("#use spec://vibevm/STATIC#root\n");
    assert_eq!(d.errors, vec![], "bare `STATIC` is not the lane path");

    let d = Directives::parse("#use spec://org.example/pkg/notes/STATIC#x\n");
    assert_eq!(d.errors, vec![], "`notes/STATIC` is not the lane path");
}
