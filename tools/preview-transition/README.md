# Preview transition checker

`python -B tools/preview-transition/check.py --transition
docs/releases/developer-preview-1-to-2/transition.json --mode capture --format json`
checks the registered transition and capture shapes, their semantic invariants,
the retained raw capture hash, and every recorded Git tree, package manifest,
and source-file identity. It reads the selected repository and writes only a
machine report to stdout.

The Python entrypoint only parses CLI arguments and sends a versioned request
to `preview-transition-check`, a `vibe-wire` binary that uses generated JTD
readers. Exit `0` means the selected stage passed; `1` means a failed request
or claim; `2` means the selected stage remains pending. A passing capture
still reports later documentation and final-product proof as pending.

The request grammar also names `contribution`, `bindings` (`roles` or `linked`),
`documentation` (`intake`, `updated`, or `verified`), `release` (`reviewed` or
`verified`), and `retired`. Their domain checks remain pending until the
corresponding implementation work lands; the checker never promotes a
currently unimplemented stage to passed.

`python3 -B tools/preview-transition/assemble.py --transition
docs/releases/developer-preview-1-to-2/transition.json --output-dir <owned-path>`
creates `MIGRATION-DRAFT.md` and `WORKLIST-DRAFT.md`. Python transports paths
to the Cargo binary `preview-transition-assemble`; generated Rust readers and
the existing semantic validators own transition and release-impact parsing.
The enclosing `vibe.toml` identifies the project, and the project layout API
resolves its specification root. Sorted immutable IDs select
`changes/<id>/release-impact.json` beneath that root.

Assembly validates all selected records before creating outputs. It rejects
unknown epochs, duplicate or mismatched IDs, dangling relations and capture
mismatches. Manifest, transition and record inputs use no-follow capabilities
and stable bounded snapshots at their observed lengths; symlinks and shared
hard-linked files refuse. Shape diagnostics do not echo arbitrary source values.
DP1, present DP2 and the optional sorted capture catalogue form one lookup.
Contribution subjects may name intermediate captures instead of the global
preview endpoints. A repeated ID must retain the same digest/commit/tree;
alternate safe retrieval locations are retained and remain unverified by
assembly. Pending DP2 allows unknown before/after subjects only as visible
`UNRESOLVED` warnings. Once DP2 is captured, unknown record capture IDs refuse
before any output publication. Additional supplied references and exact
identities appear in both draft headers, explicitly unverified by assembly.
Drafts retain scoped effects, reasons, acceptance and evidence as recorded;
assembly never verifies these claims or infers net delivery from relations.
Migration recipes, documentation mappings and independent net reconciliation
remain explicit review obligations. Prose is escaped so embedded Markdown,
HTML, line breaks and display controls cannot introduce false document structure.

The owned output directory may be created when absent. Existing output files,
directories and symlinks are refused. `vibe-safefs` pins directory capabilities
through a no-follow walk and publishes whole staged files without replacing
an occupant. A failed second publication rolls back earlier files only while
their identities still match the proofs returned by publication. A replaced
file survives; a possibly-published file without a proof stays as named
residue. Refusal reports retained residue instead of deleting unproved data.
Successful assembly exits `0`; refused input or output exits `1`. Retrying
after completed rollback produces the same bytes. No input source is modified.

Run the focused tests with `cargo test -p vibe-wire --bin
preview-transition-assemble` and `python3 -B -m unittest discover -s
tools/preview-transition/tests -p 'test_*.py'`. The Python integration suite
builds its binary once, or reuses `PREVIEW_TRANSITION_ASSEMBLER_BIN` when supplied.
