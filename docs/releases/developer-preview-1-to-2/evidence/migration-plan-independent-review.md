# C1 independent semantic review — final re-review

Read-only review of the stable migration.rs, migration/tests.rs and manifest/mod.rs exports; revised PROP-044 MANIFEST-MIGRATION-PLAN/FMT-MANIFEST; supplied independent golden vector and retained test logs. No execution, edits to product, Cargo, gates, Git writes, public operations or acceptance.

## Both concrete findings closed on actual bytes

**Frozen dependency qualification — resolved.** Revised governing source makes the exact structural boundary explicit: `[kind:]group/name`, exactly one slash, nonempty group/name, no @, at most one optional colon with the fixed six-kind vocabulary; current owner retains semantic naming/constraint validation. Actual migration.rs now calls qualified_key before dispatching scalar or table requirement values. That helper implements those precise restrictions and returns UnrecognizedShape independently of Manifest::parse_str. Thus a future semantic decoder broadening cannot silently remove the frozen structural requirement. It does not implement a replacement Group/name/version parser.

The new frozen_dependency_qualification_refuses_structural_unknowns case exercises unqualified/unknown/doc prefixes, extra or empty slash components, embedded version and multiple colons, plus each permitted kind and the unprefixed qualified form. The revised malformed_and_current_semantically_invalid_output_refuse still isolates current semantic validation. Retained exact-case logs show each selected case passed1/failed0/filtered530. These close the prior boundary finding without a broader family rerun.

**Independent r1 identity vector — resolved.** Actual independently_fixed_r1_identity_and_token_vector pins explicit complete before/after documents, byte span86..96, exact before/after tokens, and the three literal expected hashes. They match the supplied /tmp/vibevm-next-M13-C1-golden.json vector attributed to independent Python hashlib over explicit fixed bytes and source-defined big-endian framing, without production hashing-helper calls:

- Before content: 905cf0b4eb7376819f47c9f7da02fecb8920d91dd9b76b4cb3454da77fa81887.
- After content: 6798d562a0ff899a5afd793054821fc6a114f8cd55fc53118d7772e9167a44df.
- Diff: 42faf5376728e4649c80657e6b0f93e9910a107261413a5d0c7f019bf5ab27e0.

The retained golden exact-case log shows passed1/failed0/filtered531. I compared the static fixture/token/span/hash bytes and source framing; I did not independently rerun Python or Rust. The fixed oracle now detects a framing/domain/order change that relational identity tests alone would miss.

## Preserved contract fidelity

Caller selection remains explicit. Missing source/target/recipe returns named NeedsInput even for singleton remaining choices; unknown IDs/revisions and incompatible edges refuse. No product version, mode or absent epoch selects a profile. The finite registry permits only the one legacy-absent→current-absent rename and two distinct current identity edges; no epoch is inserted or removed.

Field sets, primitive shapes, package-kind/link/mode vocabularies and structural qualification are frozen before current semantic checking. Unsupported keys/sections/types, doc kind, explicit wrong epoch, variable/path and other omitted shapes refuse. A recognized shape can still fail current Manifest semantic validation; that final check does not widen recognition.

Span editing uses immutable Document and the owning decoded string/span. Source slicing proves UTF-8 boundaries. Four quote families, escaped Unicode, immediate multiline LF/CRLF, inline/header tables, comments, owner choices and original prefix/suffix bytes survive according to the exact-token contract. Decoded extra newline/space is not silently trimmed into snapshot. There is no DocumentMut/despan, serializer, newline normalization, alternate TOML/escape parser or fallback. The finite recipe emits one edit; no untested multiple-edit ordering is claimed.

SHA256 identities use explicit domain-framed byte inputs with u64be lengths; diff includes selected profiles/recipe, exact source/output and ordered field/span/token data. No clock, environment, filesystem location or Debug serialization enters identity. Public exports carry exact texts, edits, identities, typed outcomes/refusals/input requirements and checked Manifest. Reviewed imports/calls are std::ops, sha2, toml_edit and the existing pure Manifest checker; no filesystem/cache/lock/provider/LLM/publication/apply seam is called.

Diagnostic reasons omit parser dumps/source values, while diagnostic field paths intentionally contain authored key text. Future transport must not claim those paths contain no source-derived content and must bind appropriate disclosure/projection semantics.

## Evidence scope and limits

Original-tests.log explicitly identifies itself as a faithful copy of completion tool output, not a complete redirected compiler log; it clearly labels omitted initial compiler output and the pre-existing manifest warning. Its family result is12passed/0failed/518filtered. Subsequent retained logs select only the changed semantic case and two new cases, each nonzero. This is evidence reuse plus focused proof, not an asserted fresh final blanket family run. I read logs, did not execute or authenticate the harness independently.

Synthetic package cases are honestly marked synthetic. The exact historical14-byte scalar `v = "snapshot"` has retained provenance/SHA and is explicitly refused as document.v; it is not wrapped into a fabricated historical package manifest. The separate historical PackageMeta input uses in-place/bridge, not snapshot. No positive real historical authored-manifest rehearsal or complete pre-epoch corpus has been established. The governing source retains that broader rehearsal requirement; scope the current result to bounded synthetic integration plus exact historical-scalar refusal.

No further concrete implementation finding remains in the reviewed perimeter. Root retains final source/evidence review and acceptance. Writer stopped.
