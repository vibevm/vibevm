# M-01-D.1 producer candidate — Root review required

Same durable Zap actor/executor, one writer. No D2/next atom, native collaborator, staging, commit or publication. Owner documentation supplements are already saved separately and accepted by Root; this report does not reopen accepted C4.

Final observed HEAD: `4e491c67e357f483b5f10fd182a445728577a688`. Root's disjoint operational documentation commits advanced HEAD during D1. Product candidate is wholly UNSTAGED, with empty staged diff. Exact 62 changed/new source paths, full SHA256/bytes/line counts and that HEAD are in `candidate-source-hashes.json`; no Cargo manifest/lock, runtime config or schema ABI changes. Root owns source review, final host specmap, acceptance and atomic commit.

## Changes and requirement binding

| D1 obligation | Actual owner and evidence |
| --- | --- |
| Descriptor before parser/fragment dispatch | `source_query/capture/continuity/extract.rs` joins retained authored manifest, captured source-output row, selected interpretation and C3 recorded disposition/media. `vibedeps/descriptor_map.rs` propagates original declared media for Mixed/copied/copied-unconverted and target media for Converted from the same validated observations. No suffix/content sniffing. `final-factory-consumer-diagnostic.log`3/3 exercises the actual materializer→factory→FsSectionSource→compiler route, including misleading MD/XML suffixes, converted table cells and valid Doc package/XML note vocabulary. |
| Ordinary exports/raw remain ordinary | `extract.rs` returns held raw bytes independently of optional Vibe/XML-data inspection. `extract/export.rs` resolves retained authored CommonMark name/file/headings over the same captured raw span/digest, with candidate-bearing refusal. CommonMark literals, same-stem XML-data, invalid XML, invalid encoding/opaque and invalid Vibe view are actual cases in `final-workspace-extraction.log`9/9. No ordinary-resource fact view or original-source proof is minted. |
| Encoding/recipe/span basis | `RepresentationWitness` explicitly distinguishes captured bytes versus emitted Markdown and records encoding, named recipe and exact offsets within that representation. `ExtractionProvenance` retains descriptor identity/index, original/output paths, captured manifest/output digest, actual output media, local converter claim and weaker observed/record-consistent strength. Projected positions/outline sizes are never relabeled as raw offsets. |
| Shared selector/read_spec/compiler | `vibe-specdoc/src/selection.rs` builds one bounded IR index and selects exact section, paragraph/list/table/quote fact carrier, dotted path, authored Doc p01 and documentation block; unknown/ambiguous refuse with candidates. Canonical hash now uses literal exact-anchor selection over the same index. `resolver/selection.rs` consumes a held snapshot and checks the prepared output digest before selected-media parse; incumbent `embed.rs` consumes this IR, with its legacy helper structurally extracted to `embed/selection.rs`. Actual compiler/read_spec section/list/table IR parity for MD and XML is in the workspace9/9; actual Fs embed family3/3 includes missing media and changed digest refusal. |
| children=false and outline | Shared selector returns the selected subtree or just direct section blocks; `extract/export.rs::outline_spec` builds heading chains and canonical representation byte sizes directly from captured IR, without a prebuilt specmap. Actual direct-block, subtree/outline sizes and unknown candidates are in selector9/9 and workspace9/9. |
| Canonical native table-cell indexing/explain | Authored core `mdspec/cell_facts.rs` reuses existing fact grammar at body-cell/quoted carriers; `xmlspec.rs` mints corresponding body td/quote facts. Native cell location stops at its own row. Actual canonical `index::build`, `explain_text` and `explain_json` prove CC-A-4-1/CC-A-4-2 MD/XML identities; header structure and inline-code markers remain unminted. `final-core-index.log`4/4. Existing affected quote regression1/1 remains applicable with matching final body. |

Governing addressable clauses were written before implementation references: PROP070 `CLASSIFIED-SOURCE-EXTRACTION`, `SHARED-SPECDOC-SELECTION`, `SELECTION-REPRESENTATION-BASIS`, `TABLE-FACT-INDEX`; PROP035 `IR-SHARED-EXACT-SELECTION`; canonical core PROP014 `FACT-CELL-AND-QUOTE-GRAIN`. Status distinctions remain spec/plan rather than an autonomous readiness claim. Actual PROP043 dictionary and M06A descriptor baseline reports were read under exact packet corrections.

## Deciding failures and dispositions

- Canonical native index red0/1: CC-A-4-1 absent, only root indexed. Existing grammar carrier fix gives4/4. Native range red0/1 included following row5..6; incumbent own-row rule now passes4/4.
- Root literal-heading review reproduced2/2 failures: valid XML numeric/literal-dot headings lost old recipe1 hash. Exact literal lookup restores both independent old hash expectations; general dotted/literal collision refuses. Final selector9/9 and prior C3 canonical hash compatibility7/7. Recipe `sha256-specdoc-node/1` retained, not silently reinterpreted.
- Source-inferred selected one-cell table roundtrip concern was refuted1/1; no extra hash/representation rewrite.
- Wrapper-domain and terminal blank-line mistakes were test-fixture errors, not removed authored content: actual compiler compares the complete uniquely framed authored payload's parsed IR and canonical serialization. Raw logs retained for both failed attempts.
- Factory first E0308 compile and two invalid Doc package setups are recorded as my fixture failures; final valid Doc package uses the actual required DocumentsDecl. No validator change. Zero-test wrong module filters are explicit operator failures, not proof; corrected exact registration paths executed nonzero.
- Full attempt ledger is `INCIDENTS.md`; every existing raw failed log remains in this E directory. Root integrates FIXED-ISSUES.

## Final bounded commands/results

All Cargo commands ran from `/tmp`, `CARGO_TARGET_DIR=/fast/git/v/.build/c4-first-agent-target`, supported `cargo +nightly`, absolute manifest, `--locked --offline`. No simultaneous Cargo writer or cold target. Small families were bound before dispatch; exact compatibility cases use `-- --exact`.

Root manifest `R=/fast/git/v/vibevm-next/Cargo.toml`; canonical engine manifest `C=/fast/git/v/vibevm-next/vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/Cargo.toml`. The following table states the literal Cargo arguments after `cargo +nightly test --manifest-path` and before redirection:

| Manifest / arguments | Exit; passed/failed/ignored | Final raw log |
| --- | --- | --- |
| R `-p vibe-specdoc --lib selection::m_01_d_1_cases --locked --offline` | 0;9/0/0 | final-selector.log |
| R `-p vibe-specdoc --lib canonical_node --locked --offline` | 0;7/0/0 | final-canonical-compat.log |
| R `-p vibe-workspace --lib source_query::capture::continuity::extract::m_01_d_1_cases --locked --offline` | 0;9/0/0 | final-workspace-extraction.log |
| R `-p vibe-workspace --lib source_query::capture::continuity::extract::factory_cases --locked --offline` | 0;3/0/0 | final-factory-negative-bound.log |
| R `-p vibe-spec --lib embed::selection::m_01_d_1_cases --locked --offline` | 0;3/0/0 | final-spec-embed-correct-selector.log |
| R `-p vibe-spec --lib embed::tests::a_section_of_an_xml_dependency_reads_as_its_canonical_twin --locked --offline -- --exact` | 0;1/0/0 | final-legacy-embed-compat.log |
| R `-p vibe-workspace --lib source_query::capture::continuity::m_01_b_2_cases::unrelated_broken_member_does_not_poison_selected_point --locked --offline -- --exact` | 0;1/0/0 | final-world-compat-correct-selector.log |
| C `-p core-ai-native-specmap --lib index::m_01_d_1_cases --locked --offline` | 0;4/0/0 | final-core-index.log |
| C `-p core-ai-native-specmap --lib xmlspec::tests::anchored_quote_is_a_unit_while_unanchored_table_cell_stays_unminted --locked --offline -- --exact` | 0;1/0/0, reused matching final body | core-quote-affected-regression.log |

Final bounded passing corpus above:38 nonignored cases, not a workspace/full qualification claim. Test discovery mistakes are excluded. All source/log hashes are retained alongside this report; they are local evidence integrity, never original-source authentication.

Subsequent Root negative review: the existing copied-unconverted fixture now additionally binds actual installed raw output byte equality, Xml slot policy, exact retained source/output row and CopiedUnconverted issue/message/line5. Its actual compiler refusal specifically contains the Markdown parser's standalone-marker diagnostic and line5; unrelated unknown anchor/missing map cannot satisfy it. The same factory3/3 family was rerun on this final assertion body; no new production change or unrelated case.

## Generator/format checks

Exact existing generator command: `CARGO_MANIFEST_DIR=/fast/git/v/vibevm-next/xtask /fast/git/v/.build/c4-first-agent-target/debug/xtask sync-engines [--check]`, cwd `/tmp`.

Precheck exit1 retained `sync-pre-check.log`; independent full registered inventory `sync-pre-inventory.json` proved36 approved specmap src differences only. Normal generator exit0,36 operations, `sync-generator.log`; no manual vendor editing. All51 registered pairs across9 sets byte-identical afterwards, including all six FULL specmap trees (`sync-post-inventory.json`). Postcheck exit1 remains solely two unregistered Lean dirs B230 (`sync-post-check.log`); no unrelated source/Cargo/license/task metadata copied.

Touched handwritten/generated Rust `rustfmt +nightly --edition 2024 --config skip_children=true --check <exact touched paths>` exit0 (`final-touched-fmt-check.log`). All59 touched Rust files<=600, maximum582; three governing XML parse0. `git diff --check` exit0 (`diff-check.log`). `cargo +nightly fmt --manifest-path R --all -- --check` exit1, exactly13 outside-owner Diff entries retained in `final-full-fmt-check.log`; no blanket format green or baseline cleanup. Root final current compiled specmap generation remains pending review; this producer has not claimed that gate.

## Honest boundary / next action

This is the callable prepared/captured selected-world API and actual compiler #embed integration, not D2 CLI/MCP adapters or general public-channel completion. Legacy no-descriptor compiler route remains its explicitly weaker compatibility path. Existing compiler memory overlay is supplied by its caller; it is not original-source authentication. Public raw snapshot/index/record agreement never becomes SourceProven; install C2 opaque proof and C3 derivation owner remain authoritative and untouched.

VibeDoc native XML vocabulary works; unsupported transformed Doc Markdown structured reading remains unavailable under the existing C3 reader boundary, while captured raw retrieval remains available. No native/custom converter executes, no fetch/hydration/rebuild/query locks/directories/COW copies are introduced. Existing finite B2 continuity/C1 held observation limits remain; no hostile-ABA, atomic race-free or native Windows/macOS qualification claim.

Independent review dispatch is unavailable: actual `managed_work_profiles` returned `unsupported_operation` because trusted managed runtime is unconfigured and asked for reconnect, forbidden for this preprovisioned actor. No catalog-eligible Astra profile or independent launch is fabricated; no direct private/native fallback. Root supplies acceptance judgment and owns the recorded operational gap.

Next concrete step: Root review of these exact source hashes/evidence and current compiled host specmap integration, then acceptance/atomic commit if satisfied. Producer stops at this candidate, keeps the same runtime alive and does not start another atom.
