# M-01-A.3 candidate for Root review

2026-10-12. Base HEAD `6f4f3b1c7fff13a51e7a6040c4eee7a4fc9543ae`. Product candidate is unstaged. Root owns FIXED-ISSUES.md, acceptance, specmap integration and the atomic commit; the current FIXED-ISSUES.md delta is Root's disjoint edit.

## Bounded result

- `resolver.rs`/`resolver/portable.rs` distinguish absent document (`DocNotFound`), failed structural/directory/read operation (`SourceUnreadable`), case/NFC/path refusal (`UnsafeSourcePath` or `PortableSourceAlias`), and observed disappearance/change (`SourceUnstable`). Initial absence remains absent; disappearance after a successful directory check no longer becomes not-found.
- `resolver/glob.rs` no longer converts every `resolve_doc` error into a missing pattern member. Only actual `DocNotFound` excludes that member; ambiguity, unreadability and instability return the typed failure. Two installed slots with the same matched package name now return `AmbiguousPatternMember` instead of choosing the lexicographically first root. The incumbent 12-case pattern family still passes.
- `resolver/read.rs` provides a preflighted internal source reader. It checks the selected physical source through the accepted A2 portable/no-follow seam immediately before disk reading, then compares two byte reads; differing bytes or post-lookup disappearance return `SourceUnstable`. `embed.rs` uses this path for both `section_text` and compiler-facing `read_resolved_source`. Already supplied overlay bytes remain the overlay source. Valid Markdown/XML projection uses the same `vibe_specdoc::project_spec_text` function.
- `embed.rs` refuses an addressed ID appearing in `DocTree::duplicate_anchors()` before `resolve_path` can return its first occurrence. This reaches both recursive embed and `SelectedWorldPromptResolver`; no doctree source change was needed. New `resolver/m_01_a_3_cases.rs` and inline `agent/resolver.rs` tests characterize the callers.
- `PROP-070#INTERNAL-LOOKUP-FAILURES` records the exact internal failure boundary and states the two-read/race limitation. No dependency, Cargo.lock, manifest or unrelated package source changed. All touched Rust files are below 600 lines.

## Executed evidence

All commands used the same private target `/fast/git/v/.build/c4-first-agent-target`, `cargo +nightly test --locked --offline` from this checkout, explicit `--lib` targets and nonzero selectors.

| Subject | Final log | Result |
| --- | --- | --- |
| New resolver A3 corpus (absent, blocked slot, glob ambiguity/slot collision, duplicate anchor, descriptor symlink, compiler-facing content/refusals) | `final2-a3.log` | 8/8, 0 ignored |
| Deterministic read seam (bytes changed between passes through `FileResolver` preflight; non-file source) | `read-final2.log` | 2/2, 0 ignored |
| Provider prompt new refusal cases | `final2-prompt.log` | 2/2, 0 ignored |
| Existing selected cross-package provider prompt | `final2-prompt_selected.log` | 1/1, 0 ignored |
| Compiler active-format source and typed deferred frontend cases | `final2-compiler.log`, `final2-compiler_deferred.log` | 1/1 each, 0 ignored |
| Existing XML embed projection | `final2-embed_xml.log` | 1/1, 0 ignored |
| Prior A1/A2 compatibility, matching lookup body | `verified-a1.log`, `final-a2.log` | 7/7 each, 0 ignored |
| Existing pattern behavior | `final-glob.log` | 12/12, 0 ignored |
| Existing prompt self/missing and compiler embed-error replay | `verified-prompt_self.log`, `verified-prompt_missing.log`, `verified-compiler_failure.log` | 1/1 each, 0 ignored |

`package-fmt-final2.log`: package-scoped `vibe-spec`/`vibe-lifecycle` format check exit 0. `xml-check.log`: PROP-070 parsed and A3 anchor exists. `git diff --check` exit 0. `workspace-fmt-final.log`: full workspace format **exit 1, 15 unrelated hunks**, the unchanged prior red baseline; no broad gate is claimed. Root must regenerate/review specmap for the new source edge; this executor did not run it.

## Reproduced failures and operator incidents

- `resolver-before.log`: before the resolver/glob fix, 1/3; slot-as-file was falsely `PackageSlotNotFound`, and an ambiguous matched document vanished from the pattern set. After repair, the final A3 corpus passed.
- `duplicate-anchor-before.log`: exact `FsSectionSource` regression failed 0/1, returning `FIRST_ONLY`; `prompt-before.log`: provider prompt failed 1/2, also returning the first body. Both pass after the incumbent duplicate-index refusal.
- `read-helper-first.log`: my first test compile failed with E0277 because I passed generic `fs::read` directly where the injected closure needed `|path| fs::read(path)`. Corrected; `read-final2.log` is green.
- The first package-format check reported only touched formatting differences; I applied package-scoped rustfmt and `package-fmt-final2.log` passes. Full workspace format remains independently red as above.
- I requested narrow READ corrections for actual compiler owner `crates/vibe-spec/src/compiler/pass_tier/frontend.rs` and its `frontend_discovery_tests.rs`, and for `crates/vibe-spec/src/doctree.rs`. I then read a small `doctree.rs` API excerpt before Root answered. That was my packet-scope mistake; I did not edit it. Its existing `duplicate_anchors()` API let the repair stay in `embed.rs`; the doctree WRITE correction is unnecessary. Compiler call-site tracing used `rg` owner symbols (`frontend.rs` calls `source_metadata`/`read_resolved_source`) and the exact existing compiler tests named above; full read of that outside-packet owner remains pending Root's scope response. Root should confirm that trace during review.

## Limits for acceptance

The deterministic unstable test proves a change between two observed reads through the production preflight path. There is no injected end-to-end compiler or prompt race fixture, no retained-handle byte capture and no native Windows/macOS qualification; subsequent namespace races remain possible. Existing `SectionSource` and provider interfaces still carry textual reasons at their public boundary, preserving typed `ResolveError` classification at the resolver and precise cause text to those callers. The accepted A1 selected-world and legacy compatibility distinctions remain; ordinary raw resources are not promoted to Vibe facts. No full panel, mutation, public Git/release, stopped Zap+Lean, PodBay or Aeneas work ran.

## Review identifiers

SHA256: `embed.rs` `6ca2f21df1637ee54d3bd2baa935d306bbb21440c441a4a586a61b8e84b1dbdb`; `resolver.rs` `a9b790d9fecb30fee697d54db43012c326a029605638a2c297d86b0e51202774`; `glob.rs` `eb4d80b55480c890d70d6caa1ab7ed6bca0cc26b51aea8529232f566655b83a5`; `lookup.rs` `5628eccbe15a4fd7ebb2eef3babd529c325fe2b7832ccf2acf50a1b89a2a75d0`; `portable.rs` `54dce72d371063237eacafef8d509b248a1d147e01e63152d630c3805b313b9f`; `read.rs` `357a9f1245f7c97014bf70e307371a79d4321351393d9b9763c63bec4d1cf797`; `m_01_a_3_cases.rs` `e59f9f0d062a872e5688533382c413a625923d34c364af825b6dde10005d34cf`; provider resolver `d6a34239224ed740d996e4569b23b703eab3a64139bce0cb75cdc6b35ca01be0`; PROP-070 `532c0a4150e630b71e7df59b9c7e09f946258a0019ff7913b32066916863521c`. Root should re-hash after any review correction.

Next action: Root reviews the source and the stated proof limit, verifies the compiler call-site boundary and specmap, then accepts/commits A3 if sufficient. Do not start M-01-D from this candidate.
