//! The qualify cell's in-place tests, split out along the file-length
//! seam (conform `file-length`): two `#[cfg(test)]` modules verbatim.

#[cfg(test)]
mod tests {
    use crate::qualify::*;

    // ---- 1. Golden — every category, exact output ---------------------------

    #[test]
    fn golden_contribution_rewrites_every_category() {
        let slug = "org-vibevm-world--wal";
        // Exercises: a heading anchor, a paragraph fact, a list-item fact,
        // intra-links to a heading and to a fact, an unknown intra-link,
        // a fenced block carrying fake labels (incl. a (#root) that IS
        // defined — it must stay unrewritten inside the fence), an inline-code
        // span carrying a defined (#root), a full spec:// address, an @spec://
        // in-place use, and a directive line.
        let input = "\
# Heading One {#root}

##FACT-ONE The first fact.

See [the link](#root) and [the fact](#FACT-ONE) and [none](#missing).

- ##FACT-TWO A list fact.

```
# fake ##root and (#never) and (#root)
```

Inline code: `##root` and `(#root)`.

@spec://org.vibevm.world/wal/doc#root is a use.

#use spec://org.vibevm.world/wal/doc#root
";
        let expected = format!(
            "\
# Heading One {{#{slug}--root}}

##{slug}--FACT-ONE The first fact.

See [the link](#{slug}--root) and [the fact](#{slug}--FACT-ONE) and [none](#missing).

- ##{slug}--FACT-TWO A list fact.

```
# fake ##root and (#never) and (#root)
```

Inline code: `##root` and `(#root)`.

@spec://org.vibevm.world/wal/doc#root is a use.

#use spec://org.vibevm.world/wal/doc#root
"
        );

        let (out, renames) = qualify_contribution(input, "org.vibevm.world/wal");

        assert_eq!(out, expected);
        assert_eq!(
            renames
                .iter()
                .map(|r| r.original.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "FACT-ONE", "FACT-TWO"],
            "rename map is in document order"
        );
        assert_eq!(
            renames
                .iter()
                .map(|r| r.qualified.as_str())
                .collect::<Vec<_>>(),
            vec![
                "org-vibevm-world--wal--root",
                "org-vibevm-world--wal--FACT-ONE",
                "org-vibevm-world--wal--FACT-TWO",
            ]
        );
    }

    #[test]
    fn the_qualified_opener_is_qualified_too() {
        // The failure this guards against is silent by construction: an
        // unrecognised opener is not an error here, it is simply a label left
        // unqualified — and two packages sharing a fact id then collide in the
        // compiled lane with nothing said. Measured once at 466 markers
        // spliced, none of them qualified.
        let slug = "org-vibevm-world--wal";
        let input = "\
# Heading One {#root}

@fact:FACT-ONE The first fact. @status:impl/done

- @fact:FACT-TWO A list fact. @status:spec/work

See [the fact](#FACT-ONE).
";
        let expected = format!(
            "\
# Heading One {{#{slug}--root}}

@fact:{slug}--FACT-ONE The first fact. @status:impl/done

- @fact:{slug}--FACT-TWO A list fact. @status:spec/work

See [the fact](#{slug}--FACT-ONE).
"
        );

        let (out, renames) = qualify_contribution(input, "org.vibevm.world/wal");
        assert_eq!(out, expected);
        assert_eq!(
            renames
                .iter()
                .map(|r| r.original.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "FACT-ONE", "FACT-TWO"]
        );
    }

    #[test]
    fn both_openers_qualify_to_the_same_label() {
        // A half-migrated snippet must not produce two different labels for
        // what is one id.
        let legacy = qualify_contribution("##A One. @impl/done\n", "org.vibevm.world/wal");
        let qualified =
            qualify_contribution("@fact:A One. @status:impl/done\n", "org.vibevm.world/wal");
        assert_eq!(
            legacy.1.first().map(|r| r.qualified.as_str()),
            qualified.1.first().map(|r| r.qualified.as_str()),
        );
    }

    // ---- 2. Prefix-reversibility -------------------------------------------

    #[test]
    fn stripping_the_prefix_restores_the_original() {
        // Idempotence-by-construction is not claimed; instead the prefix is
        // reversible: applying the rename map in reverse (longest qualified
        // first, so a label that prefixes another is handled cleanly) restores
        // the original byte-for-byte.
        let input = "# H {#root}\n##FACT-A the statement\nSee [r](#root) and [f](#FACT-A).\n";
        let (qualified, renames) = qualify_contribution(input, "org.vibevm.world/wal");

        // The structural form first (it only borrows `qualified`): every
        // change inserts exactly `<slug>--`, and that substring is absent from
        // the input, so removing it restores the original.
        assert_eq!(
            qualified.replace("org-vibevm-world--wal--", ""),
            input,
            "every rewrite is a pure insertion of the slug prefix"
        );

        // And the map-driven form: applying the rename map in reverse (longest
        // qualified first, so a label that prefixes another is handled cleanly)
        // restores the original byte-for-byte.
        let mut sorted = renames.clone();
        sorted.sort_by_key(|r| std::cmp::Reverse(r.qualified.len()));
        let mut restored = qualified;
        for r in &sorted {
            restored = restored.replace(&r.qualified, &r.original);
        }
        assert_eq!(restored, input);
    }

    // ---- 3. Append-independence --------------------------------------------

    #[test]
    fn qualify_is_independent_of_sibling_contributions() {
        // The signature takes ONE contribution's text and its origin — no view
        // of any sibling — so qualifying A is byte-identical whether or not B
        // exists. The property is enforced structurally: there is no
        // cross-input for it to depend on.
        let a = "# A {#root}\n##A-FACT x\n[link](#root)\n";
        let b = "# B {#root}\n##B-FACT y\n[link](#root)\n";

        let (qa, ra) = qualify_contribution(a, "org.vibevm.world/aaa");
        // B is qualified in a separate call A can never observe.
        let _ = qualify_contribution(b, "org.vibevm.world/bbb");
        let (qa2, ra2) = qualify_contribution(a, "org.vibevm.world/aaa");

        assert_eq!(qa, qa2);
        assert_eq!(ra, ra2);
        // A's labels carry A's origin, never B's.
        assert!(qa.contains("org-vibevm-world--aaa--root"));
        assert!(!qa.contains("org-vibevm-world--bbb"));
    }

    // ---- 4. Slug edge cases ------------------------------------------------

    #[test]
    fn origin_slug_edge_cases() {
        // Dotted group: dots -> `-`, the group/name `/` -> `--`.
        assert_eq!(origin_slug("org.vibevm.world/wal"), "org-vibevm-world--wal");
        // A single host-like token (no `/`): slug is the lowercased token, no
        // joiner.
        assert_eq!(origin_slug("vibevm"), "vibevm");
        // A ` [shared by ...]` provenance suffix is dropped (normal_seed's rule).
        assert_eq!(
            origin_slug("org.vibevm.world/wal [shared by a/b]"),
            "org-vibevm-world--wal"
        );
        // Always lowercased.
        assert_eq!(origin_slug("Org.VIBEVM.World/Wal"), "org-vibevm-world--wal");
    }

    #[test]
    fn single_token_origin_qualifies_labels_without_a_joiner() {
        // A no-`/` origin mints a joiner-less slug, so the qualified form is
        // `<token>--<label>` (one `--`, the label joiner — no group/name join).
        let (out, renames) = qualify_contribution("# H {#root}\n", "vibevm");
        assert_eq!(out, "# H {#vibevm--root}\n");
        assert_eq!(renames[0].qualified, "vibevm--root");
    }

    // ---- 5. Empty rename map -----------------------------------------------

    #[test]
    fn contribution_with_no_labels_is_unchanged_with_empty_map() {
        let input = "# A plain heading\n\nSome prose with no labels or links.\n";
        let (out, renames) = qualify_contribution(input, "org.vibevm.world/wal");
        assert!(renames.is_empty());
        assert_eq!(out, input);
    }

    // ---- R3 pinned: a `###` run is never a fact id -------------------------

    #[test]
    fn h3_run_and_glued_triple_hash_are_not_fact_ids() {
        // R3: a spaced `### Heading` is a heading (its anchor IS qualified);
        // a glued `###GLUED` is neither heading nor fact. Neither mints a fact.
        let (out, renames) =
            qualify_contribution("### Heading {#h}\n###GLUED prose\n", "vibevm/wal");
        assert_eq!(out, "### Heading {#vibevm--wal--h}\n###GLUED prose\n");
        assert_eq!(
            renames
                .iter()
                .map(|r| r.original.as_str())
                .collect::<Vec<_>>(),
            vec!["h"]
        );
    }

    #[test]
    fn mid_line_double_hash_is_not_a_fact_definition() {
        // A `##X` that is not the line's lead token is prose, not a definition.
        let (out, renames) =
            qualify_contribution("lead text then ##NOT-A-FACT here\n", "vibevm/wal");
        assert_eq!(out, "lead text then ##NOT-A-FACT here\n");
        assert!(renames.is_empty());
    }

    #[test]
    fn unknown_intra_link_is_left_untouched() {
        // A (#y) whose target this contribution does not define is not ours to
        // resolve — left byte-identical.
        let (out, renames) = qualify_contribution("[x](#missing)\n", "vibevm/wal");
        assert_eq!(out, "[x](#missing)\n");
        assert!(renames.is_empty());
    }
}

#[cfg(test)]
mod cell_fact_tests {
    use crate::qualify::*;

    /// K6.5 cell facts: a `@fact:ID` opening a table cell is a definition in
    /// the shared namespace — qualified like any lead fact, one rename per
    /// id, and a mid-cell `@fact:`-looking token stays prose.
    #[test]
    fn cell_facts_qualify_like_lead_facts() {
        let text = concat!(
            "| field | meaning |\n",
            "|---|---|\n",
            "| @fact:CELL-A the a cell | plain text |\n",
            "| prose about @fact:NOT-A-DEF | @fact:CELL-B b cell |\n",
        );
        let (out, renames) = qualify_contribution(text, "org.demo/pkg");
        assert!(
            out.contains("| @fact:org-demo--pkg--CELL-A the a cell |"),
            "{out}"
        );
        assert!(
            out.contains("| @fact:org-demo--pkg--CELL-B b cell |"),
            "{out}"
        );
        // Mid-cell sigil is prose, untouched.
        assert!(out.contains("prose about @fact:NOT-A-DEF"), "{out}");
        let originals: Vec<&str> = renames.iter().map(|r| r.original.as_str()).collect();
        assert_eq!(originals, vec!["CELL-A", "CELL-B"], "{renames:?}");
    }

    /// A `<quote>` fact projects as `> @fact:ID …` — a definition, qualified
    /// like a lead fact (the source-mirrors daily-loop shape).
    #[test]
    fn blockquote_facts_qualify_like_lead_facts() {
        let text = "> @fact:QUOTED-LAW never clobber silently.
";
        let (out, renames) = qualify_contribution(text, "org.demo/pkg");
        assert!(out.contains("> @fact:org-demo--pkg--QUOTED-LAW"), "{out}");
        assert_eq!(renames.len(), 1, "{renames:?}");
        assert_eq!(renames[0].original, "QUOTED-LAW");
    }
}
