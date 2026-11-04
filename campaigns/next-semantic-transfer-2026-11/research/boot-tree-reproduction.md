# Nested committed boot attribution: actual CLI reproduction

**Observed final result:7 tests executed,5passed,2 intended regression failures, cargo exit101.** Fourteen actual `vibe tree --json --path <synthetic-project>` invocations:13exit0 with valid v1 schema,1exit1 for an unterminated XML comment. All14 project byte/path snapshots remained equal. This is a frozen TEST-ONLY candidate; no production fix, schema change or acceptance claim.

Target `/fast/git/wt/boot-artifact-tree`, branch `fix/boot-artifact-tree`, HEAD/base `ee3031c5150403b7cf693c77d467eb01c3e8a67b`. Own non-symlink target. Linux execution. Prototype `7fe6376ccef7f321302a8f18610fcb0c9cc2686f` was read only through the Git object for `crates/vibe-workspace/src/boot_debugger/static_summary.rs`; its dirty checkout was untouched and its code was not executed/copied. No network/services, agents, Git mutations, main writes or common-app worktree access.

## Exact valid occurrence observations

A/B/C below mean `spec://org.probe/project/common/{A,B,C}#root`. Lines are **one-based file-relative inclusive marker boundaries**. Tests compare complete occurrence tuples after sorting solely for comparison; original CLI array order is preserved in the JSON observations. The opening/source order shown below is recorded, not imposed as a future output-order policy.

| Case | Intended valid occurrences in opening order | Actual CLI occurrences in returned order | Result |
|---|---|---|---|
| Single-level LF | A[3..5] | A[3..5] | control PASS |
| Same tape with CRLF | A[3..5] | A[3..5] | control PASS |
| Real public expansion owner | A[2..12], B[6..10] | B[6..10] | outer A lost, intended RED |
| Authored3levels + second contribution | A[2..8], B[3..7], C[4..6], second B[11..13] | C[4..6], second B[11..13] | A and first B lost, intended RED |
| Same-address siblings | A[2..4], A[6..8] | A[2..4], A[6..8] | distinct occurrence control PASS |
| XML c1 provenance | A[3..5] | A[3..5] | control PASS; exact decoded origin/path retained |

The compiler-generated case uses actual `FsSectionSource(FileResolver::new(...))` to read synthetic A.md/B.md, then public `vibe_spec::expand_embeds("#embed …A#root")`. It is not an authored imitation of nested expansion or a tree-parser reimplementation. A synthetic `vibe:static` wrapper around that exact expanded payload makes a committed artifact fixture for tree. The line oracle finds four known, unique literal markers independently; it does not call the production tree scanner or infer matching pairs with the same algorithm. The authored3level fixture has fixed literal line expectations. This is not a full install/boot-composition run.

The c1 control uses the real codec with proper `<!-- vibe:c1 ... -->` framing and logical origin `org.probe/a--b-%雪`, path `docs/a--b-%雪.xml`. Both decode exactly. CRLF validation here covers line semantics, not exact contribution byte accounting.

## Malformed tapes: characterization only

These cases have no assertions selecting a new malformed-marker policy. Successful outputs still validate against the existing v1 schema; every case checks the project snapshot.

| Authored case | Observed result |
|---|---|
| Open A, close B | exit0; **B[2..4]**, identity taken from closing marker although opener was A |
| Open A, open B, close A, close B | exit0; **A[3..5]**, span begins at B opener and adopts A closer |
| Unclosed A opener (XML comment itself complete) | exit0; no embed spans |
| Stray closing marker | exit0; no embed spans |
| Empty closing address | exit0; A[2..4] |
| Unterminated XML comment | exit1; CLI error; no successful tree document |
| Open in one contribution, close in next | exit0; neither contribution gets an embed span |

Complete stdout/stderr, artifact text, numbered lines, expected valid tuples and original JSON array order are in reproduction.json. These observations establish current behavior only; omission versus diagnostic versus refusal remains root's decision.

## Additional fenced-literal discriminator (separate from prior13)

At root's request one further compiler-generated case was added after preserving the original13-case run as `before-fenced-*`. A.md contains a fenced code sample with:

```text
<!-- embed: spec://org.probe/project/common/LITERAL#sample -->
#embed spec://org.probe/project/common/DO_NOT_RESOLVE#root
```

The XML comment is complete, but its literal logical embed opening has no matching closing example. The real B directive sits **outside** that fence.

Observed: `Directives::parse(A.md)` returns only the actual B directive. FsSectionSource + expand_embeds succeeds without trying to resolve DO_NOT_RESOLVE; the complete fenced sample survives **byte-for-byte**. Generated tape has real outer A markers at2/16, literal example opening at6 inside fence5..8, and real B markers10/14. Tree exits0 and returns only B[10..14]. No assertion treats the quoted marker as real provenance or selects its future attribution policy.

Source-backed implication, not a tested implementation proposal: a strict raw matching stack would see the literal opening on top when the real A closes and could falsely reject this legitimate generated document. The canonical existing Markdown fence owner is `crates/vibe-spec/src/doctree/fence.rs`: `FenceTracker::{classify, snapshot, from_snapshot}` and `fence_mask`/`mask_from`, all pub(crate), reexported crate-locally by doctree.rs. `Directives::parse` is public and consumes that mask (`directives.rs:91`), but its public result carries directives/uses/errors/aliases, **not** a reusable external fence mask. This is an API boundary for root design, not justification for a second lexer or whole boot-debugger import. XML-specific code examples, fenced static-contribution markers and other malformed combinations were not characterized here.

## Cause and evidence strength

Actual main `tree/artifacts.rs::scan_embeds` keeps one Option: inner openings overwrite outer ones and close addresses may replace opener identity. That mechanism directly explains the observed lost ancestors and crossed identities. The existing unit test named `attributes_a_nested_embed_span` contains only a single pair and checks merely start<end; it does not exercise nesting or exact boundaries. Schema validity also cannot catch lost semantic occurrences: the defective outputs remain valid v1 documents.

Pinned prototype source uses a matching stack and emits on close, but that is source evidence only. Its raw scanner also does not use the fence owner, and omission of malformed cases does not establish a chosen policy. No prototype graph, operation service or measurement framework was transferred. Precise body-byte measurement and a new explain query remain outside this attribution probe.

## Commands, errors and artifact identity

Exact final command (cwd the worktree):

```text
CARGO_TARGET_DIR=/fast/git/wt/boot-artifact-tree/target cargo test --offline --locked -p vibe-cli --test tree_nested_attribution -- --nocapture --test-threads=1
```

Final wall time5.72s, exit101;7selected,5pass,2fail,0ignored/filtered. The binary is invoked via the existing UserScratch/vibe_test_support convention with isolated user settings/cache. No real repository boot artifact was written. Read-only evidence is full project path/directory/byte snapshot equality, not a syscall trace or proof about timestamps/permissions. Synthetic fixtures are removed at test teardown.

Initial run compiled successfully but had3PASS/3FAIL: I mistakenly used the codec's **unframed payload** as the XML comment. The third failure was my fixture defect, not a product regression. Initial log/command/test bytes/observations are retained as `initial-*`; only c1 framing was corrected, yielding4PASS/2RED over the original6tests/13CLI observations. The fenced follow-up then produced the final7tests/14observations. No Rust compiler errors or production changes occurred.

Scoped rustfmt --check and git diff --check both exit0 (`checks.json`). Changed nonignored set is exactly **one new test file**; tracked production diff empty. Source/test/test-executable/CLI hashes and both successful/malformed observations are in reproduction.json. Cargo compiled the CLI in the own target during the initial run and compiled the amended test in each follow-up; executable hashes are exact final bytes, not an mtime-only inference. No whole-workspace suite, TUI interaction, prototype runtime or native Windows/macOS claim.

- Test `crates/vibe-cli/tests/tree_nested_attribution.rs`: SHA256 `7552bdd97164da6e55e0e95cddce12aa6bdf586ffcfaf4f0867fa7c0f536c61b`.
- Binary `/fast/git/wt/boot-artifact-tree/target/debug/deps/tree_nested_attribution-25d82159b9ace0cd`: SHA256 `84f43658a6fe81f0bbb74b130416c61532ff26ca5787be626b6fa5eb7c601dc0`.
- Binary `/fast/git/wt/boot-artifact-tree/target/debug/vibe`: SHA256 `34812e95927c10f6e4412a7d56ea4dcab78e91202704f30ea2bf98bd7dd05b14`.
- Raw final log: `/tmp/nst-boot-tree-probe.log`; exact command: `/tmp/nst-boot-tree-command.json`.
- Full structured report: `/tmp/nst-boot-tree-reproduction.json`; standalone observations: `/tmp/nst-boot-tree-observations.json`.
- Original13-case run: `/tmp/nst-boot-tree-before-fenced-{probe.log,command.json,observations.json,test.rs}`.
- Initial XML fixture mistake: `/tmp/nst-boot-tree-initial-{probe.log,command.json,test.rs}` and `observations-initial.json`.

Frozen with no live build/executable/fd users of this worktree. Root chooses marker/order/error/fence design, acceptance and eventual preservation/removal; this worker makes no production fix or completion claim.
